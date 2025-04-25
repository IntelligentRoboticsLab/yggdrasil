use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Instant,
};

use re_control_comms::{
    protocol::{ViewerMessage, control::ViewerControlMessage},
    viewer::ControlViewerHandle,
};
use rerun::external::{ecolor::Color32, egui, re_ui::UiExt};

use crate::{re_control_view::ControlViewerData, resource::RobotResources};

use super::view_section;

const LAST_UPDATE_COLOR: Color32 = Color32::from_gray(100);

#[derive(Default)]
pub struct ResourcesState {
    pub resources: RobotResources,
    pub last_resource_update: Option<Instant>,
    pub focused_resources: HashMap<String, bool>,
}

pub fn resource_ui(
    ui: &mut egui::Ui,
    viewer_data: Arc<RwLock<ControlViewerData>>,
    handle: &ControlViewerHandle,
) {
    view_section(ui, "Resources".to_string(), |ui| {
        resource_display_and_manage_ui(ui, viewer_data, handle);
    });
}

fn resource_display_and_manage_ui(
    ui: &mut egui::Ui,
    viewer_data: Arc<RwLock<ControlViewerData>>,
    handle: &ControlViewerHandle,
) {
    // Shows the last resource update in milliseconds
    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
        let last_resource_update = resource_update_time_ago(Arc::clone(&viewer_data));
        ui.label(
            egui::RichText::new(format!("Last updated: {last_resource_update}"))
                .monospace()
                .color(LAST_UPDATE_COLOR),
        );
    });

    // Sort the names to keep the resources in a fixed order
    let mut resource_names: Vec<_>;
    {
        let Ok(locked_data) = &mut viewer_data.read() else {
            tracing::error!("Failed to lock viewer data");
            return;
        };

        resource_names = locked_data
            .resources_state
            .resources
            .0
            .keys()
            .cloned()
            .collect();
    }
    resource_names.sort();

    if resource_names.is_empty() {
        ui.vertical_centered_justified(|ui| {
            ui.warning_label("No resources available");
        });
    }

    for name in resource_names {
        let Ok(locked_data) = &mut viewer_data.write() else {
            ui.vertical_centered_justified(|ui| {
                ui.warning_label("Not able to access viewer data");
            });
            tracing::error!("Failed to lock viewer data");
            return;
        };

        if let Some(resource_data) = locked_data.resources_state.resources.0.get_mut(&name) {
            let followup_action =
                add_editable_resource(ui, &name, resource_data, Arc::clone(&viewer_data));
            if let Some(action) = followup_action {
                if let Err(error) = handle.send(action) {
                    tracing::error!(?error, "Failed to send message");
                }
            }
        }
    }
}

fn resource_update_time_ago(viewer_data: Arc<RwLock<ControlViewerData>>) -> String {
    let Ok(locked_data) = viewer_data.read() else {
        tracing::error!("Failed to lock states");
        return "unknown".to_string();
    };

    let Some(time_ago) = locked_data
        .resources_state
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
    viewer_data: Arc<RwLock<ControlViewerData>>,
) -> Option<ViewerMessage> {
    let mut followup_action = None;

    let Ok(locked_data) = &mut viewer_data.write() else {
        ui.vertical_centered_justified(|ui| {
            ui.warning_label("Not able to access viewer data");
        });
        tracing::error!("Failed to lock viewer data");
        return None;
    };

    let changed_resources = &mut locked_data.resources_state.focused_resources;

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
            if ui
                .button(egui::RichText::new("Override resource").size(12.0))
                .clicked()
            {
                followup_action = Some(ViewerMessage::ViewerControlMessage(
                    ViewerControlMessage::UpdateResource {
                        resource_name: resource_name.to_string(),
                        value: resource_data.to_string(),
                    },
                ));

                if let Some(changed_resource) = changed_resources.get_mut(resource_name) {
                    *changed_resource = false;
                }
            }

            if ui
                .button(egui::RichText::new("Override config").size(12.0))
                .clicked()
            {}
        });
    });

    followup_action
}
