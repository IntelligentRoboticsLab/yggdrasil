use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::{Arc, RwLock},
};

use re_control_comms::{
    protocol::{RobotMessage, CONTROL_PORT},
    viewer::ControlViewer,
};
use rerun::external::egui;

use crate::{
    connection::ConnectionState, state::{HandleState, SharedHandleState},
};

/// This is the ui in the selection ui (the ui "on the side") for making a connection to a robot
pub(crate) fn connection_selection_ui<T: HandleState + Default + Send + Sync + 'static>(
    ui: &mut egui::Ui,
    connection_state: &mut ConnectionState,
    state_data: Arc<RwLock<T>>,
) {
    egui::Grid::new("re_control selection ui")
        .num_columns(2)
        .spacing([20.0, 10.0])
        .show(ui, |ui| {
            team_number_selection(ui, &mut connection_state.team_number);
            ui.end_row();

            wired_connection_selection(ui, &mut connection_state.wired_connection);
            ui.end_row();

            robot_connection_selection(ui, connection_state);
            ui.end_row();

            robot_connect_button(ui, connection_state, state_data);
            ui.end_row();
        });
}

fn team_number_selection(ui: &mut egui::Ui, team_number_state: &mut u8) {
    ui.label("Team number");
    ui.add(egui::DragValue::new(team_number_state));
}

fn wired_connection_selection(ui: &mut egui::Ui, wired_state: &mut bool) {
    ui.label("Connection type");
    ui.horizontal(|ui| {
        if ui.radio(!*wired_state, "Wireless").clicked() {
            *wired_state = false;
        }
        if ui.radio(*wired_state, "Wired").clicked() {
            *wired_state = true;
        }
    });
}

fn robot_connection_selection(ui: &mut egui::Ui, connection_state: &mut ConnectionState) {
    let team_number = connection_state.team_number;
    let wired = connection_state.wired_connection;
    let selected_robot = connection_state.robot_from_state();

    let selected_robot_config = &mut connection_state.selected_robot_config;
    let possible_robot_connections = &connection_state.possible_robot_connections;

    ui.label("Robot");

    let selected_robot_ip = if selected_robot_config.number == 0 {
        Ipv4Addr::LOCALHOST
    } else {
        selected_robot.ip()
    };

    egui::ComboBox::from_id_salt("control viewer connection")
        .selected_text(format!("{} - {}", selected_robot.name, selected_robot_ip))
        .show_ui(ui, |ui| {
            for robot_config in possible_robot_connections {
                let robot_ip = if robot_config.number == 0 {
                    Ipv4Addr::LOCALHOST
                } else {
                    robot_config.clone().to_robot(team_number, wired).ip()
                };

                ui.selectable_value(
                    selected_robot_config,
                    robot_config.clone(),
                    format!("{} - {}", robot_config.name, robot_ip),
                );
            }
        });
}

fn robot_connect_button<T: HandleState + Default + Send + Sync + 'static>(
    ui: &mut egui::Ui,
    connection_state: &mut ConnectionState,
    state_data: Arc<RwLock<T>>,
) {
    ui.label("Connect");

    if ui.button("Connect").clicked() {
        // If the local "robot" was chosen the localhost ip address is used
        let robot_ip = if connection_state.selected_robot_config.number == 0 {
            Ipv4Addr::LOCALHOST
        } else {
            connection_state.robot_from_state().ip()
        };

        let socket_addr = SocketAddrV4::new(robot_ip, CONTROL_PORT);
        let control_viewer = ControlViewer::from(socket_addr);

        state_data.reset();

        // Add a handler for the `ControlViewer` before it runs. This is to
        // make sure we do not miss any message send at the beginning of a
        // connection
        control_viewer
            .add_handler(Box::new(move |msg: &RobotMessage| {
                Arc::clone(&state_data).handle_message(&msg);
            }))
            .expect("Failed to add handler");

        let handle = control_viewer.run();
        // Replace old handle with the new `ControlViewerHandle`
        connection_state.handle = handle;
    }
}
