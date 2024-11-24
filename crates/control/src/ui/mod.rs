pub mod style;

use std::sync::{Arc, RwLock};

use re_viewer::external::egui::{self, Frame};
use rerun::external::ecolor::Color32;

use crate::{
    connection::{
        protocol::ViewerMessage,
        viewer::ControlViewerHandle,
    },
    control::ControlStates,
};

pub fn add_editable_resource(
    ui: &mut egui::Ui,
    resource_name: &String,
    resource_data: &mut String,
    states: Arc<RwLock<ControlStates>>,
    button_frame_style: Frame,
) -> Option<ViewerMessage> {
    let mut followup_action = None;

    let mut states = states.write().expect("Failed to lock states");
    let changed_resources = &mut states.focused_resources;

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

        ui.horizontal(|ui| {
            // Button to override a resource on the robot from rerun
            button_frame_style.show(ui, |ui| {
                if ui
                    .add(egui::Button::new(
                        egui::RichText::new("Override resource").size(12.0),
                    ))
                    .clicked()
                {
                    followup_action = Some(ViewerMessage::UpdateResource(
                        resource_name.to_string(),
                        resource_data.to_string(),
                    ));

                    if let Some(changed_resource) = changed_resources.get_mut(resource_name) {
                        *changed_resource = false;
                    }
                }
            });

            button_frame_style.show(ui, |ui| {
                if ui
                    .add(egui::Button::new(
                        egui::RichText::new("Override config").size(12.0),
                    ))
                    .clicked()
                {}
            })
        });

        ui.add_space(5.0);
    });

    followup_action
}

pub fn debug_resources_ui(
    ui: &mut egui::Ui,
    states: Arc<RwLock<ControlStates>>,
    handle: &ControlViewerHandle,
) {
    ui.vertical(|ui| {
        let debug_enabled_systems_view = &mut states
            .write()
            .expect("Failed to read lock states")
            .debug_enabled_systems_view;

        for system_name in &debug_enabled_systems_view.key_sequence {
            let enabled = debug_enabled_systems_view
                .debug_enabled_systems
                .systems
                .get_mut(system_name)
                .unwrap();
            if ui.checkbox(enabled, system_name).changed() {
                let message =
                    ViewerMessage::UpdateEnabledDebugSystem(system_name.clone(), *enabled);
                handle
                    .send(message)
                    .expect("Failed to send update debug system message");
            };
        }
    });
}
