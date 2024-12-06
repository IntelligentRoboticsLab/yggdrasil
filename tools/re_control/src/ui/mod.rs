pub mod style;

use std::{
    ops::RangeInclusive,
    sync::{Arc, RwLock},
};

use heimdall::CameraPosition;
use nalgebra::Vector3;
use re_viewer::external::{
    egui::{self, Frame},
    re_ui::UiExt,
};
use rerun::external::ecolor::Color32;
use style::{FrameStyleMap, LAST_UPDATE_COLOR};

use re_control_comms::{protocol::ViewerMessage, viewer::ControlViewerHandle};

use crate::control::ControlStates;

pub const SIDE_PANEL_WIDTH: f32 = 400.0;
pub const PANEL_TOP_PADDING: f32 = 10.0;

const DEGREE_SUFFIX: &str = "°";
const DEGREE_RANGE: RangeInclusive<f32> = -180.0..=180.0;
const SLIDER_STEP_SIZE: f64 = 0.01;

pub fn resource_ui(
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

pub fn debug_resources_ui(
    ui: &mut egui::Ui,
    states: Arc<RwLock<ControlStates>>,
    handle: &ControlViewerHandle,
) {
    ui.vertical(|ui| {
        let Ok(locked_states) = &mut states.write() else {
            ui.centered_and_justified(|ui| {
                ui.warning_label("Not able to access viewer states");
            });
            tracing::error!("Failed to lock states");
            return;
        };

        let debug_enabled_systems_view = &mut locked_states.debug_enabled_systems_view;
        let key_sequence = &debug_enabled_systems_view.key_sequence;

        for system_name in key_sequence {
            let Some(enabled) = debug_enabled_systems_view
                .debug_enabled_systems
                .systems
                .get_mut(system_name)
            else {
                ui.warning_label(format!("System `{}` does not exist", system_name));
                continue;
            };

            if ui.checkbox(enabled, system_name).changed() {
                let message = ViewerMessage::UpdateEnabledDebugSystem {
                    system_name: system_name.clone(),
                    enabled: *enabled,
                };

                if let Err(error) = handle.send(message) {
                    tracing::error!(?error, "Failed to send update debug system message")
                }
            };
        }
    });
}

pub fn camera_calibration_parameters_ui(
    ui: &mut egui::Ui,
    states: Arc<RwLock<ControlStates>>,
    handle: &ControlViewerHandle,
) {
    let Ok(locked_states) = &mut states.write() else {
        ui.centered_and_justified(|ui| {
            ui.warning_label("Not able to access viewer states");
        });
        tracing::error!("Failed to lock states");
        return;
    };

    let camera_state = &mut locked_states.camera_state;
    let camera_position = &mut camera_state.current_position;

    // Selectable buttons to choose the camera position
    ui.horizontal(|ui| {
        ui.selectable_value(camera_position, CameraPosition::Top, "Top");
        ui.selectable_value(camera_position, CameraPosition::Bottom, "Bottom");
    });

    let camera_config = &mut camera_state.config;
    let camera_settings = match camera_position {
        CameraPosition::Top => &mut camera_config.top,
        CameraPosition::Bottom => &mut camera_config.bottom,
    };

    let extrinsic_rotation = &mut camera_settings.extrinsic_rotation;
    {
        let current_extrinsic_rotation = extrinsic_rotation.current_mut();
        egui::Grid::new("Top camera extrinsic rotations")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                for (index, rotation_axis) in ["Pitch", "Roll", "Yaw"].iter().enumerate() {
                    extrinsic_rotation_slider(
                        ui,
                        rotation_axis,
                        index,
                        current_extrinsic_rotation,
                        *camera_position,
                        handle,
                    );
                    ui.end_row();
                }
            });
    }

    if ui.button("Restore original").clicked() {
        extrinsic_rotation.restore_from_original();

        let msg = ViewerMessage::CameraExtrinsic {
            camera_position: *camera_position,
            extrinsic_rotation: *extrinsic_rotation.current(),
        };
        if let Err(error) = handle.send(msg) {
            tracing::error!(?error, "Failed to send message");
        }
    }
}

fn extrinsic_rotation_slider(
    ui: &mut egui::Ui,
    rotation_axis: &str,
    rotation_axis_idx: usize,
    rotations: &mut Vector3<f32>,
    camera_position: CameraPosition,
    handle: &ControlViewerHandle,
) {
    ui.label(rotation_axis);
    if ui
        .add(
            egui::Slider::new(rotations.index_mut(rotation_axis_idx), DEGREE_RANGE)
                .suffix(DEGREE_SUFFIX)
                .smart_aim(false)
                .drag_value_speed(SLIDER_STEP_SIZE)
                .step_by(SLIDER_STEP_SIZE),
        )
        .changed()
    {
        let msg = ViewerMessage::CameraExtrinsic {
            camera_position,
            extrinsic_rotation: *rotations,
        };
        if let Err(error) = handle.send(msg) {
            tracing::error!(?error, "Failed to send message");
        }
    };
}
