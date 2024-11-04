use std::{collections::HashMap, time::Instant};

use async_std::net::TcpStream;
use futures::{
    channel::mpsc,
    io::{ReadHalf, WriteHalf},
};
use re_viewer::external::{
    eframe,
    egui::{self, ScrollArea},
};

use yggdrasil::core::{
    control::{
        receive::{ControlClientMessage, ControlReceiver},
        transmit::{ControlHostMessage, ControlSender},
    },
    debug::debug_system::DebugEnabledSystems,
};

use crate::{
    connection::{
        connect::RobotConnection,
        receive::{handle_message, receive_messages, HandleMessageStatus},
        transmit::send_messages,
    },
    resource::RobotResources,
    ui::{
        add_editable_resource, debug_resources_ui,
        style::{FrameStyleMap, LAST_UPDATE_COLOR},
    },
};

#[derive(Default)]
pub struct ControlStates {
    pub robot_resources: RobotResources,
    pub focused_resources: HashMap<String, bool>,
    pub last_resource_update: Option<Instant>,
    pub debug_enabled_systems_view: DebugEnabledSystemsView,
}

#[derive(Default)]
pub struct DebugEnabledSystemsView {
    pub debug_enabled_systems: DebugEnabledSystems,
    pub key_sequence: Vec<String>,
}

impl From<DebugEnabledSystems> for DebugEnabledSystemsView {
    fn from(debug_enabled_systems: DebugEnabledSystems) -> Self {
        let mut key_sequence: Vec<_> = debug_enabled_systems.systems.keys().cloned().collect();
        key_sequence.sort();
        Self {
            debug_enabled_systems,
            key_sequence,
        }
    }
}

pub struct Control {
    app: re_viewer::App,
    states: ControlStates,
    message_receiver: Option<ControlReceiver<ControlHostMessage>>,
    message_sender: ControlSender<ControlClientMessage>,
    frame_styles: FrameStyleMap,
}

impl eframe::App for Control {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Store viewer state on disk
        self.app.save(storage);
    }

    /// Called whenever we need repainting, which could be 60 Hz.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Feels weird to put the `handle_message` in a function
        // that is executed when ui needs repainting
        if let Some(mut message_receiver) = self.message_receiver.take() {
            self.message_receiver = match handle_message(&mut message_receiver, &mut self.states) {
                HandleMessageStatus::Stopped => None,
                HandleMessageStatus::Continue => Some(message_receiver),
            }
        }

        // First add our panel(s):
        egui::SidePanel::right("Resource manipulation")
            .default_width(400.0)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    self.ui(ui);
                });
            });

        self.app.update(ctx, frame);
    }
}

impl Control {
    pub fn new(app: re_viewer::App, robot_connection: RobotConnection) -> Self {
        let receiver = Control::listen_for_robot_messages(robot_connection.reader);
        let sender = Control::setup_send_messages_to_robot(robot_connection.writer);

        Control {
            app,
            states: ControlStates::default(),
            message_receiver: Some(receiver),
            message_sender: sender,
            frame_styles: FrameStyleMap::default(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.vertical_centered(|ui| {
            ui.strong("Resource panel");
        });
        ui.separator();

        ui.horizontal(|ui| {
            // Manual resource refresh button
            // ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
            //     self.frame_styles
            //         .get_or_default("refresh_button".to_string())
            //         .show(ui, |ui| {
            //             if ui
            //                 .add(egui::Button::new(egui::RichText::new("Refresh").size(18.0)))
            //                 .clicked()
            //             {
            //                 let request = ClientRequest::RobotState;
            //                 let bytes = bincode::serialize(&request).into_diagnostic().unwrap();
            //                 send_request(self.ws.clone(), bytes).unwrap();
            //             }
            //         });
            // });

            ui.add_space(ui.available_width());

            // Shows the last resource update in miliseconds
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                let last_resource_update = {
                    match self.states.last_resource_update {
                        Some(time) => time.elapsed().as_millis(),
                        None => 0,
                    }
                };

                ui.label(
                    egui::RichText::new(format!(
                        "Last updated: {:>4} ms ago",
                        last_resource_update
                    ))
                    .monospace()
                    .color(LAST_UPDATE_COLOR),
                );
            });
        });

        ui.add_space(10.0);

        // Sort the names to keep the resources in a fixed order
        // let mut resource_names: Vec<String> = {
        //     let robot_resources = self.states.robot_resources;
        //     robot_resources.0.keys().cloned().collect()
        // };
        // resource_names.sort();
        let mut resource_names: Vec<_> = self.states.robot_resources.0.keys().cloned().collect();
        resource_names.sort();

        let resources = &mut self.states.robot_resources;

        for name in resource_names.into_iter() {
            if let Some(data) = resources.0.get_mut(&name) {
                let followup_action = add_editable_resource(
                    ui,
                    &name,
                    data,
                    &mut self.states.focused_resources,
                    self.frame_styles
                        .get_or_default("override_button".to_string()),
                );
                if let Some(action) = followup_action {
                    self.message_sender.tx.unbounded_send(action).unwrap();
                }
            }
        }

        ui.separator();
        ui.horizontal(|ui| {
            debug_resources_ui(
                ui,
                &mut self.states.debug_enabled_systems_view,
                &self.message_sender,
            )
        });
    }

    fn listen_for_robot_messages(
        reader: ReadHalf<TcpStream>,
    ) -> ControlReceiver<ControlHostMessage> {
        let (reader_tx, reader_rx) = mpsc::unbounded::<ControlHostMessage>();
        tokio::spawn(async move {
            receive_messages(reader, reader_tx).await;
        });

        ControlReceiver { rx: reader_rx }
    }

    fn setup_send_messages_to_robot(
        writer: WriteHalf<TcpStream>,
    ) -> ControlSender<ControlClientMessage> {
        let (writer_tx, writer_rx) = mpsc::unbounded::<ControlClientMessage>();
        tokio::spawn(async move {
            send_messages(writer, writer_rx).await;
        });
        ControlSender { tx: writer_tx }
    }
}
