use std::sync::{Arc, RwLock};

use crate::control::ControlStates;
use re_control_comms::{protocol::ViewerMessage, viewer::ControlViewerHandle};
use re_viewer::external::{
    egui::{self, Frame},
    re_ui::{list_item, UiExt},
};
use rerun::external::ecolor::Color32;

use super::style::{FrameStyleMap, LAST_UPDATE_COLOR};

pub fn resource_ui(
    ui: &mut egui::Ui,
    states: Arc<RwLock<ControlStates>>,
    handle: &ControlViewerHandle,
    frame_styles: &FrameStyleMap,
) {
    list_item::list_item_scope(ui, "Control resources", |ui| {
        ui.spacing_mut().item_spacing.y = ui.ctx().style().spacing.item_spacing.y;
        ui.section_collapsing_header("Resources")
            .default_open(false)
            .show(ui, |ui| {
                resource_display_and_manage_ui(ui, states, handle, frame_styles);
            })
    });
}

fn resource_display_and_manage_ui(
    ui: &mut egui::Ui,
    states: Arc<RwLock<ControlStates>>,
    handle: &ControlViewerHandle,
    frame_styles: &FrameStyleMap,
) {
    // Shows the last resource update in milliseconds
    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
        let last_resource_update = resource_update_time_ago(Arc::clone(&states));
        ui.label(
            egui::RichText::new(format!("Last updated: {last_resource_update}"))
                .monospace()
                .color(LAST_UPDATE_COLOR),
        );
    });

    // Sort the names to keep the resources in a fixed order
    let mut resource_names: Vec<_>;
    {
        let states = states.read().expect("Failed to lock states");
        resource_names = states.robot_resources.0.keys().cloned().collect();
    }
    resource_names.sort();

    if resource_names.is_empty() {
        ui.centered_and_justified(|ui| {
            ui.warning_label("No resources available");
        });
    }

    for name in resource_names {
        let mut current_states = states.write().expect("Failed to lock states");
        if let Some(data) = current_states.robot_resources.0.get_mut(&name) {
            let followup_action = add_editable_resource(
                ui,
                &name,
                data,
                Arc::clone(&states),
                frame_styles.get_or_default("override_button".to_string()),
            );
            if let Some(action) = followup_action {
                if let Err(error) = handle.send(action) {
                    tracing::error!(?error, "Failed to send message");
                }
            }
        }
    }
}

fn resource_update_time_ago(states: Arc<RwLock<ControlStates>>) -> String {
    let Ok(locked_states) = states.read() else {
        tracing::error!("Failed to lock states");
        return "unknown".to_string();
    };

    let Some(time_ago) = locked_states
        .last_resource_update
        .map(|time| time.elapsed().as_millis())
    else {
        return "unknown".to_string();
    };

    format!("{:>4} ms ago", time_ago)
}

fn add_editable_resource(
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
        ui.add_space(5.0);

        let mut resource_name_color = Color32::GRAY;
        if let Some(true) = changed_resources.get(resource_name) {
            resource_name_color = Color32::LIGHT_RED;
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
                    followup_action = Some(ViewerMessage::UpdateResource {
                        resource_name: resource_name.to_string(),
                        value: resource_data.to_string(),
                    });

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
    });

    followup_action
}
