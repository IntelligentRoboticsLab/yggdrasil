use std::{
    ops::RangeInclusive,
    sync::{Arc, RwLock},
};

use re_control_comms::{protocol::ViewerMessage, viewer::ControlViewerHandle};
use rerun::external::{egui, re_ui::UiExt};

use crate::re_control_view::ControlViewerData;

use super::view_section;

const DEGREE_RANGE: RangeInclusive<f32> = 0.25..=0.45;
const SLIDER_STEP_SIZE: f64 = 0.001;

#[derive(Default)]
pub struct ChromaticityState {
    pub green_threshold: f32,
}

pub fn chromaticity_ui(
    ui: &mut egui::Ui,
    states: Arc<RwLock<ControlViewerData>>,
    handle: &ControlViewerHandle,
) {
    view_section(ui, "Chromaticity".to_string(), |ui| {
        ui.spacing_mut().item_spacing.y = ui.ctx().style().spacing.item_spacing.y;

        let Ok(locked_states) = &mut states.write() else {
            ui.centered_and_justified(|ui| {
                ui.warning_label("Not able to access viewer states");
            });
            tracing::error!("Failed to lock states");
            return;
        };

        threshold_slider(
            ui,
            &mut locked_states.chromaticity_state.green_threshold,
            handle,
        );
    });
}

fn threshold_slider(
    ui: &mut egui::Ui,
    chromaticity_threshold: &mut f32,
    handle: &ControlViewerHandle,
) {
    ui.label("Green threshold");
    if ui
        .add(
            egui::Slider::new(chromaticity_threshold, DEGREE_RANGE)
                .smart_aim(false)
                .drag_value_speed(SLIDER_STEP_SIZE)
                .step_by(SLIDER_STEP_SIZE),
        )
        .changed()
    {
        let msg = ViewerMessage::Chromaticity {
            green_threshold: *chromaticity_threshold,
        };
        if let Err(error) = handle.send(msg) {
            tracing::error!(?error, "Failed to send chromaticity threshold");
        }
    };
}
