use std::{
    ops::RangeInclusive,
    sync::{Arc, RwLock},
};

use yggdrasil_rerun_comms::{
    protocol::{
        ViewerMessage,
        control::{FieldColorConfig, ViewerControlMessage},
    },
    viewer::ControlViewerHandle,
};
use rerun::external::{
    egui::{self, Color32},
    re_log,
    re_ui::UiExt,
};

use crate::yggdrasil_rerun_view::ControlViewerData;

use super::view_section;

const YHS_RANGE: RangeInclusive<f32> = 0.0..=255.0;
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

        let mut config = locked_states.field_color.config.clone();
        let mut changed = false;

        ui.add_space(5.0);
        ui.label(
            egui::RichText::new("Thresholds")
                .color(Color32::WHITE)
                .size(14.0),
        );
        ui.horizontal(|ui| {
            ui.columns(3, |columns| {
                columns[0].vertical(|ui| {
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
                            "Max field hue",
                            &mut config.max_field_hue,
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
                            "Max field luminance",
                            &mut config.max_field_luminance,
                            YHS_RANGE,
                            YHS_SLIDE_STEP_SIZE,
                        );
                });

                columns[1].vertical(|ui| {
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
                            "Max white saturation",
                            &mut config.max_white_saturation,
                            YHS_RANGE,
                            YHS_SLIDE_STEP_SIZE,
                        );
                });

                columns[2].vertical(|ui| {
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
                });
            })
        });

        ui.separator();
        ui.add_space(3.0);
        ui.label(
            egui::RichText::new("Chromaticity")
                .color(Color32::WHITE)
                .size(14.0),
        );

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
            locked_states.field_color.config = config;
            if let Err(err) = handle.send(ViewerMessage::ViewerControlMessage(
                ViewerControlMessage::FieldColor {
                    config: locked_states.field_color.config.clone(),
                },
            )) {
                re_log::warn!("Failed to send field color update message: {err}");
            }
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
