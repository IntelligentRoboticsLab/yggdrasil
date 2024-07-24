use miette::{IntoDiagnostic, Result};
use re_smart_channel::Receiver;
use re_viewer::{
    external::{
        eframe::{self, NativeOptions},
        egui,
    },
    StartupOptions,
};
use rerun::log::LogMsg;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard},
};
use tokio::io;
use yggdrasil::core::control::receive::ClientRequest;

use crate::connection::TcpConnection;

// This is used for analytics, if the `analytics` feature is on in `Cargo.toml`
const APP_ENV: &str = "My Wrapper";

const WINDOW_TITLE: &str = "Seidr";

#[derive(Default)]
pub struct SeidrStates {
    pub robot_resources: Arc<Mutex<RobotResources>>,
    pub focussed_resource: Arc<Mutex<Option<String>>>,
}

#[derive(Default, Debug)]
pub struct RobotResources(HashMap<String, String>);

impl RobotResources {
    fn update_resources(
        &mut self,
        updated_state_msg: RobotStateMsg,
        focussed_resource: MutexGuard<Option<String>>,
    ) -> Result<()> {
        let updated_resource_map = updated_state_msg.0;
        println!("\nFocussed on resource: {:?}\n", focussed_resource);

        for (name, updated_data) in updated_resource_map.into_iter() {
            if let Some(data) = self.0.get_mut(&name) {
                // Do not update a resource if user is focussed on it
                if let Some(focussed_resource) = focussed_resource.as_ref() {
                    if name == *focussed_resource {
                        continue;
                    }
                }
                *data = updated_data;
            } else {
                self.0.insert(name, updated_data);
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RobotStateMsg(HashMap<String, String>);

pub struct Seidr {
    app: re_viewer::App,
    connection: TcpConnection,
    states: SeidrStates,
}

impl eframe::App for Seidr {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Store viewer state on disk
        self.app.save(storage);
    }

    /// Called whenever we need repainting, which could be 60 Hz.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // First add our panel(s):
        egui::SidePanel::right("my_side_panel")
            .default_width(400.0)
            .show(ctx, |ui| {
                self.ui(ui);
            });

        // Now show the Rerun Viewer in the remaining space:
        self.app.update(ctx, frame);
    }
}

impl Seidr {
    pub fn new(app: re_viewer::App, connection: TcpConnection) -> Self {
        Seidr {
            app,
            connection,
            states: SeidrStates::default(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.vertical_centered(|ui| {
            ui.strong("My custom panel");
        });
        ui.separator();

        if ui
            .button(egui::RichText::new("Refresh").size(20.0))
            .clicked()
        {
            let request = ClientRequest::RobotState;
            let bytes = bincode::serialize(&request).into_diagnostic().unwrap();
            self.connection.send_request(bytes).unwrap();
        }

        let mut resource_names: Vec<String> = {
            let locked_robot_resources = self.states.robot_resources.lock().unwrap();
            locked_robot_resources.0.keys().cloned().collect()
        };
        // Sort the names to keep the resources at a fixed order
        resource_names.sort();

        let resources = self.states.robot_resources.clone();
        let focussed_resource = self.states.focussed_resource.clone();

        let mut locked_focussed_resource = focussed_resource.lock().unwrap();
        {
            let mut locked_resource_map = resources.lock().unwrap();

            for name in resource_names.into_iter() {
                if let Some(data) = locked_resource_map.0.get_mut(&name) {
                    let followup_action =
                        add_editable_resource(ui, &name, data, &mut locked_focussed_resource);
                    if let Some(action) = followup_action {
                        match action {
                            EditableResourceAction::ResourceUpdate(bytes) => self.connection.send_request(bytes).unwrap(),
                        };
                    }
                }
            }
        }
    }

    pub fn listen_for_robot_response(&mut self) {
        let mut msg = [0; 4096];
        let rs = self.connection.rs.clone();
        let robot_resources = self.states.robot_resources.clone();
        let focussed_resource = self.states.focussed_resource.clone();

        tokio::spawn(async move {
            loop {
                rs.readable().await.into_diagnostic().unwrap();
                match rs.try_read(&mut msg) {
                    Ok(0) => break, // Connection closed
                    Ok(num_bytes) => {
                        match bincode::deserialize::<RobotStateMsg>(&msg[..num_bytes])
                            .into_diagnostic()
                        {
                            Ok(robot_state_msg) => {
                                println!("Server response: {:?}", robot_state_msg);
                                let mut locked_robot_resources = robot_resources.lock().unwrap();
                                let locked_focussed_resource = focussed_resource.lock().unwrap();
                                let _ = locked_robot_resources
                                    .update_resources(robot_state_msg, locked_focussed_resource);
                            }
                            Err(e) => {
                                println!("Failed to deserialize server response; err = {:?}", e);
                                break;
                            }
                        }
                    }
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                    Err(e) => {
                        println!("Failed to read from socket; err = {:?}", e);
                        break;
                    }
                }
            }
        });
    }
}

enum EditableResourceAction {
    ResourceUpdate(Vec<u8>),
}

fn add_editable_resource(
    ui: &mut egui::Ui,
    name: &String,
    data: &mut String,
    focussed_resource: &mut Option<String>,
) -> Option<EditableResourceAction> {
    let mut followup_action = None;

    ui.vertical(|ui| {
        ui.label(egui::RichText::new(name).heading()); // Use heading style for label
        ui.add_space(10.0); // Add space between the label and the text editor
        let multiline_text = ui.add(
            egui::TextEdit::multiline(data)
                .font(egui::TextStyle::Monospace) // for cursor height
                .code_editor()
                .desired_rows(1)
                .lock_focus(true)
                .desired_width(f32::INFINITY),
        );
        if ui
            .button(egui::RichText::new("Override resource").size(10.0))
            .clicked()
        {
            let request = ClientRequest::ResourceUpdate(name.to_owned(), data.to_owned());
            let bytes = bincode::serialize(&request).into_diagnostic().unwrap();
            followup_action = Some(EditableResourceAction::ResourceUpdate(bytes));
        }

        // Logic to remember which resource not to update when focussed
        if multiline_text.has_focus() {
            *focussed_resource = Some(name.to_owned());
        } else {
            if let Some(resource_name) = focussed_resource {
                if resource_name == name {
                    *focussed_resource = None;
                }
            }
        }
    });

    followup_action
}

pub struct App {
    rx: Receiver<LogMsg>,
    startup_options: StartupOptions,
    native_options: NativeOptions,
    robot_connection: TcpConnection,
}

impl App {
    pub fn new(
        rx: Receiver<LogMsg>,
        startup_options: StartupOptions,
        native_options: NativeOptions,
        robot_connection: TcpConnection,
    ) -> Self {
        App {
            rx,
            startup_options,
            native_options,
            robot_connection,
        }
    }

    pub fn run(self) -> Result<()> {
        eframe::run_native(
            WINDOW_TITLE,
            self.native_options,
            Box::new(move |cc| {
                let _re_ui = re_viewer::customize_eframe_and_setup_renderer(cc);

                let mut rerun_app = re_viewer::App::new(
                    re_viewer::build_info(),
                    &re_viewer::AppEnvironment::Custom(APP_ENV.to_string()),
                    self.startup_options,
                    cc.egui_ctx.clone(),
                    cc.storage,
                );
                rerun_app.add_receiver(self.rx);

                let mut seidr = Seidr::new(rerun_app, self.robot_connection);
                seidr.listen_for_robot_response();
                Ok(Box::new(seidr))
            }),
        )
        .into_diagnostic()?;

        Ok(())
    }
}
