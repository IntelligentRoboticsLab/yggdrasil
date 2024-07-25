use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use miette::IntoDiagnostic;
use re_viewer::external::{
    eframe,
    egui::{self, ScrollArea},
};
use rerun::external::ecolor::Color32;
use yggdrasil::core::control::receive::ClientRequest;

use crate::{
    connection::{self, TcpConnection},
    resource::RobotResources,
};

const COLOR_BUTTON_BACKGROUND: Color32 = Color32::from_rgb(20, 20, 20);

#[derive(Default)]
pub struct SeidrStates {
    pub robot_resources: Arc<Mutex<RobotResources>>,
    pub focused_resources: Arc<Mutex<HashMap<String, bool>>>,
}

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
        egui::SidePanel::right("Resource manipulation")
            .default_width(400.0)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    self.ui(ui);
                });
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
            .button(
                egui::RichText::new("Refresh")
                    .size(20.0)
                    .background_color(COLOR_BUTTON_BACKGROUND),
            )
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
        let focused_resources = self.states.focused_resources.clone();
        let mut locked_focused_resources = focused_resources.lock().unwrap();

        // {
        let mut locked_resource_map = resources.lock().unwrap();

        for name in resource_names.into_iter() {
            if let Some(data) = locked_resource_map.0.get_mut(&name) {
                let followup_action =
                    add_editable_resource(ui, &name, data, &mut locked_focused_resources);
                if let Some(action) = followup_action {
                    match action {
                        EditableResourceAction::ResourceUpdate(bytes) => {
                            self.connection.send_request(bytes).unwrap()
                        }
                    };
                }
            }
            // }
        }
    }

    pub fn listen_for_robot_responses(&mut self) {
        let rs = self.connection.rs.clone();
        let robot_resources = self.states.robot_resources.clone();
        let focused_resource = self.states.focused_resources.clone();

        let handle_robot_message = move |robot_state_msg| {
            let mut locked_robot_resources = robot_resources.lock().unwrap();
            let locked_focused_resource = focused_resource.lock().unwrap();
            let _ =
                locked_robot_resources.update_resources(robot_state_msg, locked_focused_resource);
        };

        connection::receiving_responses(rs, handle_robot_message)
    }
}

enum EditableResourceAction {
    ResourceUpdate(Vec<u8>),
}

fn add_editable_resource(
    ui: &mut egui::Ui,
    resource_name: &String,
    resource_data: &mut String,
    changed_resources: &mut HashMap<String, bool>,
) -> Option<EditableResourceAction> {
    let mut followup_action = None;

    ui.vertical(|ui| {
        let mut resource_name_color = Color32::GRAY;
        if let Some(changed_resource) = changed_resources.get(resource_name) {
            if *changed_resource {
                resource_name_color = Color32::LIGHT_RED;
            }
        }
        ui.label(
            egui::RichText::new(resource_name)
                .heading()
                .color(resource_name_color),
        );
        ui.add_space(10.0);

        // Editable text block for the resource data
        let multiline_text = ui.add(
            egui::TextEdit::multiline(resource_data)
                .font(egui::TextStyle::Monospace)
                .code_editor()
                .desired_rows(1)
                .lock_focus(true)
                .desired_width(f32::INFINITY),
        );

        if multiline_text.changed() {
            if let Some(changed_resource) = changed_resources.get_mut(resource_name) {
                *changed_resource = true;
            }
        }

        // Button to override a resource on the robot from Seidr
        if ui
            .button(
                egui::RichText::new("Override resource")
                    .size(15.0)
                    .background_color(COLOR_BUTTON_BACKGROUND),
            )
            .clicked()
        {
            let request =
                ClientRequest::ResourceUpdate(resource_name.to_owned(), resource_data.to_owned());
            let bytes = bincode::serialize(&request).into_diagnostic().unwrap();
            followup_action = Some(EditableResourceAction::ResourceUpdate(bytes));

            if let Some(changed_resource) = changed_resources.get_mut(resource_name) {
                *changed_resource = false;
            }
        }

        // // Logic to remember which resource not to update when focussed
        // if multiline_text.has_focus() {
        //     *focused_resource = Some(resource_name.to_owned());
        // } else if let Some(focused_resource_name) = focused_resource {
        //     if resource_name == focused_resource_name {
        //         *focused_resource = None;
        //     }
        // }
    });

    followup_action
}
