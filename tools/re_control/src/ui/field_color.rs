use std::{
    ops::RangeInclusive,
    sync::{Arc, RwLock},
};

use re_control_comms::{
    protocol::{FieldColorConfig, ViewerMessage},
    viewer::ControlViewerHandle,
};
use rerun::external::{egui, re_ui::UiExt};

use crate::re_control_view::ControlViewerData;

use super::view_section;

const YHS_RANGE: RangeInclusive<f32> = 0.0..=25.0;
const YHS_SLIDE_STEP_SIZE: f64 = 1.0;

const DEGREE_RANGE: RangeInclusive<f32> = 0.25..=0.45;
const SLIDER_STEP_SIZE: f64 = 0.001;

#[derive(Default)]
pub struct FieldColorState {
    pub config: FieldColorConfig,
}

pub fn field_color_ui(
    ui: &mut egui::Ui,
    states: Arc<RwLock<ControlViewerData>>,
    handle: &ControlViewerHandle,
) {
    view_section(ui, "Field Color".to_string(), |ui| {
        ui.spacing_mut().item_spacing.y = ui.ctx().style().spacing.item_spacing.y;

        let Ok(locked_states) = &mut states.write() else {
            ui.centered_and_justified(|ui| {
                ui.warning_label("Not able to access viewer states");
            });
            tracing::error!("Failed to lock states");
            return;
        };

        let config = &mut locked_states.field_color.config;
        let mut changed = false;
        ui.heading("Thresholds");
        changed = changed
            || threshold_slider(
                ui,
                "Max field hue",
                &mut config.max_field_hue,
                YHS_RANGE,
                YHS_SLIDE_STEP_SIZE,
            );
        changed = changed
            || threshold_slider(
                ui,
                "Max field luminance",
                &mut config.max_field_luminance,
                YHS_RANGE,
                YHS_SLIDE_STEP_SIZE,
            );
        changed = changed
            || threshold_slider(
                ui,
                "Min field hue",
                &mut config.min_field_hue,
                YHS_RANGE,
                YHS_SLIDE_STEP_SIZE,
            );
        changed = changed
            || threshold_slider(
                ui,
                "Min field saturation",
                &mut config.min_field_saturation,
                YHS_RANGE,
                YHS_SLIDE_STEP_SIZE,
            );

        changed = changed
            || threshold_slider(
                ui,
                "Max white saturation",
                &mut config.max_white_saturation,
                YHS_RANGE,
                YHS_SLIDE_STEP_SIZE,
            );
        changed = changed
            || threshold_slider(
                ui,
                "Min white luminance",
                &mut config.min_white_luminance,
                YHS_RANGE,
                YHS_SLIDE_STEP_SIZE,
            );

        changed = changed
            || threshold_slider(
                ui,
                "Max black saturation",
                &mut config.max_black_saturation,
                YHS_RANGE,
                YHS_SLIDE_STEP_SIZE,
            );
        changed = changed
            || threshold_slider(
                ui,
                "Max black luminance",
                &mut config.max_black_luminance,
                YHS_RANGE,
                YHS_SLIDE_STEP_SIZE,
            );

        ui.heading("Chromaticity");
        changed = changed
            || threshold_slider(
                ui,
                "Red threshold",
                &mut config.red_chromaticity_threshold,
                DEGREE_RANGE,
                SLIDER_STEP_SIZE,
            );
        changed = changed
            || threshold_slider(
                ui,
                "Green threshold",
                &mut config.green_chromaticity_threshold,
                DEGREE_RANGE,
                SLIDER_STEP_SIZE,
            );

        changed = changed
            || threshold_slider(
                ui,
                "Blue threshold",
                &mut config.blue_chromaticity_threshold,
                DEGREE_RANGE,
                SLIDER_STEP_SIZE,
            );

        if changed {
            handle
                .send(ViewerMessage::FieldColor {
                    config: locked_states.field_color.config.clone(),
                })
                .expect("failed to send message")
        }
    });
}

fn threshold_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    step_size: f64,
) -> bool {
    ui.label(label);
    ui.add(
        egui::Slider::new(value, range)
            .smart_aim(false)
            .drag_value_speed(step_size)
            .step_by(step_size),
    )
    .changed()
}
