use std::{
    ops::RangeInclusive,
    sync::{Arc, RwLock},
};

use heimdall::CameraPosition;
use nalgebra::Vector3;
use re_control_comms::{protocol::ViewerMessage, viewer::ControlViewerHandle};
use re_viewer::external::{
    egui,
    re_ui::{list_item, UiExt},
};

use crate::control::ControlStates;

const DEGREE_SUFFIX: &str = "°";
const DEGREE_RANGE: RangeInclusive<f32> = -20.0..=20.0;
const SLIDER_STEP_SIZE: f64 = 0.01;

pub fn camera_calibration_ui(
    ui: &mut egui::Ui,
    states: Arc<RwLock<ControlStates>>,
    handle: &ControlViewerHandle,
) {
    list_item::list_item_scope(ui, "Control camera calibration", |ui| {
        ui.spacing_mut().item_spacing.y = ui.ctx().style().spacing.item_spacing.y;
        ui.section_collapsing_header("Camera calibration parameters")
            .default_open(true)
            .show(ui, |ui| {
                camera_extrinsic_rotation_ui(ui, states, handle);
            })
    });
}

fn camera_extrinsic_rotation_ui(
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
        egui::Grid::new("Camera extrinsic rotations")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                for (index, rotation_axis) in ["Roll", "Pitch", "Yaw"].iter().enumerate() {
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
