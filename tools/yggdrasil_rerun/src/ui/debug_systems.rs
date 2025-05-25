use std::sync::{Arc, RwLock};

use rerun::external::{egui, re_ui::UiExt};

use yggdrasil_rerun_comms::{
    debug_system::DebugEnabledSystems,
    protocol::{ViewerMessage, control::ViewerControlMessage},
    viewer::ControlViewerHandle,
};

use crate::yggdrasil_rerun_view::ControlViewerData;

use super::view_section;

#[derive(Default)]
pub struct DebugEnabledState {
    debug_enabled_systems: DebugEnabledSystems,
    key_sequence: Vec<String>,
}

impl DebugEnabledState {
    pub fn update(&mut self, debug_enabled_systems: DebugEnabledSystems) {
        let mut key_sequence: Vec<_> = debug_enabled_systems.systems.keys().cloned().collect();
        key_sequence.sort();
        self.debug_enabled_systems = debug_enabled_systems;
        self.key_sequence = key_sequence;
    }
}

pub fn debug_enabled_systems_ui(
    ui: &mut egui::Ui,
    viewer_data: Arc<RwLock<ControlViewerData>>,
    handle: &ControlViewerHandle,
) {
    view_section(ui, "Debug enabled systems".to_string(), |ui| {
        debug_enabled_systems_control_ui(ui, viewer_data, handle);
    });
}

fn debug_enabled_systems_control_ui(
    ui: &mut egui::Ui,
    viewer_data: Arc<RwLock<ControlViewerData>>,
    handle: &ControlViewerHandle,
) {
    ui.vertical(|ui| {
        let Ok(locked_data) = &mut viewer_data.write() else {
            ui.vertical_centered_justified(|ui| {
                ui.warning_label("Not able to access viewer data");
            });
            tracing::warn!("Failed to lock viewer data");
            return;
        };

        let debug_enabled_state = &mut locked_data.debug_enabled_state;

        let key_sequence = &debug_enabled_state.key_sequence;

        if key_sequence.is_empty() {
            ui.vertical_centered_justified(|ui| {
                ui.warning_label("No debug systems available");
            });
            return;
        }

        for system_name in key_sequence {
            let Some(enabled) = debug_enabled_state
                .debug_enabled_systems
                .systems
                .get_mut(system_name)
            else {
                ui.warning_label(format!("System `{}` does not exist", system_name));
                continue;
            };

            if ui.checkbox(enabled, system_name).changed() {
                let message = ViewerMessage::ViewerControlMessage(
                    ViewerControlMessage::UpdateEnabledDebugSystem {
                        system_name: system_name.clone(),
                        enabled: *enabled,
                    },
                );

                if let Err(error) = handle.send(message) {
                    tracing::error!(?error, "Failed to send update debug system message")
                }
            };
        }
    });
}
