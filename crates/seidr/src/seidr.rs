use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};

use miette::IntoDiagnostic;
use re_viewer::external::{
    eframe,
    egui::{self, Frame, ScrollArea},
};
use rerun::external::ecolor::Color32;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use yggdrasil::core::control::receive::ClientRequest;

use crate::{
    connection::{self, send_request},
    resource::RobotResources,
    style::{FrameStyleMap, LAST_UPDATE_COLOR},
};

#[derive(Default)]
pub struct SeidrStates {
    pub robot_resources: Arc<Mutex<RobotResources>>,
    pub focused_resources: Arc<Mutex<HashMap<String, bool>>>,
    pub last_resource_update: Arc<Mutex<Option<Instant>>>,
}

pub struct Seidr {
    app: re_viewer::App,
    ws: Arc<OwnedWriteHalf>,
    states: SeidrStates,
    frame_styles: FrameStyleMap,
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

        self.app.update(ctx, frame);
    }
}

impl Seidr {
    pub fn new(app: re_viewer::App, ws: Arc<OwnedWriteHalf>) -> Self {
        Seidr {
            app,
            ws,
            states: SeidrStates::default(),
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
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                self.frame_styles
                    .get_or_default("refresh_button".to_string())
                    .show(ui, |ui| {
                        if ui
                            .add(
                                egui::Button::new(egui::RichText::new("Refresh").size(18.0)),
                            )
                            .clicked()
                        {
                            let request = ClientRequest::RobotState;
                            let bytes = bincode::serialize(&request).into_diagnostic().unwrap();
                            send_request(self.ws.clone(), bytes).unwrap();
                        }
                    });
            });

            ui.add_space(ui.available_width());

            // Shows the last resource update in miliseconds
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                let last_resource_update = {
                    let lock = self.states.last_resource_update.lock().unwrap();
                    match *lock {
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
        let mut resource_names: Vec<String> = {
            let locked_robot_resources = self.states.robot_resources.lock().unwrap();
            locked_robot_resources.0.keys().cloned().collect()
        };
        resource_names.sort();

        let resources = self.states.robot_resources.clone();
        let focused_resources = self.states.focused_resources.clone();
        let mut locked_focused_resources = focused_resources.lock().unwrap();

        let mut locked_resource_map = resources.lock().unwrap();

        for name in resource_names.into_iter() {
            if let Some(data) = locked_resource_map.0.get_mut(&name) {
                let followup_action = add_editable_resource(
                    ui,
                    &name,
                    data,
                    &mut locked_focused_resources,
                    self.frame_styles
                        .get_or_default("override_button".to_string()),
                );
                if let Some(action) = followup_action {
                    match action {
                        EditableResourceAction::ResourceUpdate(bytes) => {
                            send_request(self.ws.clone(), bytes).unwrap()
                        }
                    };
                }
            }
        }
    }

    pub fn listen_for_robot_responses(&mut self, rs: OwnedReadHalf) {
        let robot_resources = self.states.robot_resources.clone();
        let focused_resource = self.states.focused_resources.clone();
        let last_resource_update = self.states.last_resource_update.clone();

        let handle_robot_message = move |robot_state_msg| {
            let mut locked_robot_resources = robot_resources.lock().unwrap();
            let locked_focused_resource = focused_resource.lock().unwrap();
            let _ =
                locked_robot_resources.update_resources(robot_state_msg, locked_focused_resource);
        };

        connection::receiving_responses(rs, last_resource_update, handle_robot_message);
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
    button_frame_style: Frame,
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
        ui.add_space(2.0);

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

        ui.add_space(2.0);

        // Button to override a resource on the robot from Seidr
        button_frame_style.show(ui, |ui| {
            if ui
                .add(egui::Button::new(
                    egui::RichText::new("Override resource").size(12.0),
                ))
                .clicked()
            {
                let request = ClientRequest::ResourceUpdate(
                    resource_name.to_owned(),
                    resource_data.to_owned(),
                );
                let bytes = bincode::serialize(&request).into_diagnostic().unwrap();
                followup_action = Some(EditableResourceAction::ResourceUpdate(bytes));

                if let Some(changed_resource) = changed_resources.get_mut(resource_name) {
                    *changed_resource = false;
                }
            }
        });
        ui.add_space(5.0);
    });

    followup_action
}
