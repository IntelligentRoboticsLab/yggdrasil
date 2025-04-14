use std::{
    ops::RangeInclusive,
    sync::{Arc, RwLock},
};

use heimdall::CameraPosition;
use nalgebra::Vector3;
use re_control_comms::{
    protocol::{ViewerMessage, control::ViewerControlMessage},
    viewer::ControlViewerHandle,
};
use rerun::external::{egui, re_ui::UiExt};

use crate::{re_control_view::ControlViewerData, state::TrackedState};

use super::view_section;

const DEGREE_SUFFIX: &str = "°";
const DEGREE_RANGE: RangeInclusive<f32> = -20.0..=20.0;
const SLIDER_STEP_SIZE: f64 = 0.01;

pub struct CameraState {
    pub current_position: CameraPosition,
    pub config: CameraConfig,
}

impl Default for CameraState {
    fn default() -> Self {
        Self {
            current_position: CameraPosition::Top,
            config: Default::default(),
        }
    }
}

#[derive(Default)]
pub struct CameraConfig {
    pub top: CameraSettings,
    pub bottom: CameraSettings,
}

#[derive(Default)]
pub struct CameraSettings {
    pub extrinsic_rotation: TrackedState<Vector3<f32>>,
}

pub fn camera_calibration_ui(
    ui: &mut egui::Ui,
    viewer_data: Arc<RwLock<ControlViewerData>>,
    handle: &ControlViewerHandle,
) {
    view_section(ui, "Camera calibration".to_string(), |ui| {
        camera_extrinsic_rotation_ui(ui, viewer_data, handle);
    });
}

fn camera_extrinsic_rotation_ui(
    ui: &mut egui::Ui,
    viewer_data: Arc<RwLock<ControlViewerData>>,
    handle: &ControlViewerHandle,
) {
    let Ok(locked_data) = &mut viewer_data.write() else {
        ui.vertical_centered_justified(|ui| {
            ui.warning_label("Not able to access viewer data");
        });
        tracing::warn!("Failed to lock viewer data");
        return;
    };

    let camera_state = &mut locked_data.camera_state;
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
        extrinsic_rotation.restore_original();

        let msg = ViewerMessage::ViewerControlMessage(ViewerControlMessage::CameraExtrinsic {
            camera_position: *camera_position,
            extrinsic_rotation: *extrinsic_rotation.current(),
        });
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
        let msg = ViewerMessage::ViewerControlMessage(ViewerControlMessage::CameraExtrinsic {
            camera_position,
            extrinsic_rotation: *rotations,
        });
        if let Err(error) = handle.send(msg) {
            tracing::error!(?error, "Failed to send message");
        }
    };
}
