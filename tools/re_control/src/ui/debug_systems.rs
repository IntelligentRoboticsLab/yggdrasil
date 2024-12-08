use std::sync::{Arc, RwLock};

use re_viewer::external::{
    egui,
    re_ui::{list_item, UiExt},
};

use re_control_comms::{protocol::ViewerMessage, viewer::ControlViewerHandle};

use crate::control::ControlStates;

pub fn debug_enabled_systems_ui(
    ui: &mut egui::Ui,
    states: Arc<RwLock<ControlStates>>,
    handle: &ControlViewerHandle,
) {
    list_item::list_item_scope(ui, "Control debug enabled systems", |ui| {
        ui.spacing_mut().item_spacing.y = ui.ctx().style().spacing.item_spacing.y;
        ui.section_collapsing_header("Debug system controls")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    debug_enabled_systems_control_ui(ui, states, handle);
                });
            })
    });
}

fn debug_enabled_systems_control_ui(
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
