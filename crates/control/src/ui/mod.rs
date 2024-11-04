pub mod style;

use std::collections::HashMap;

use re_viewer::external::{
    egui::{self, Frame},
    re_ui::UiExt,
};
use rerun::external::ecolor::Color32;

use yggdrasil::core::control::{receive::ControlClientMessage, transmit::ControlSender};

use crate::control::DebugEnabledSystemsView;

pub fn add_editable_resource(
    ui: &mut egui::Ui,
    resource_name: &String,
    resource_data: &mut String,
    changed_resources: &mut HashMap<String, bool>,
    button_frame_style: Frame,
) -> Option<ControlClientMessage> {
    let mut followup_action = None;

    ui.vertical(|ui| {
        let mut resource_name_color = Color32::GRAY;
        if let Some(changed_resource) = changed_resources.get(resource_name) {
            if *changed_resource {
                resource_name_color = Color32::LIGHT_RED;
            }
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
                    followup_action = Some(ControlClientMessage::UpdateResource(
                        resource_name.to_string(),
                        resource_data.to_string(),
                    ));

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

        ui.add_space(5.0);
    });

    followup_action
}

pub fn debug_resources_ui(
    ui: &mut egui::Ui,
    debug_enabled_systems_view: &mut DebugEnabledSystemsView,
    message_sender: &ControlSender<ControlClientMessage>,
) {
    ui.vertical(|ui| {
        for system_name in &debug_enabled_systems_view.key_sequence {
            ui.horizontal(|ui| {
                let enabled = debug_enabled_systems_view
                    .debug_enabled_systems
                    .systems
                    .get_mut(system_name)
                    .unwrap();
                ui.label(system_name);
                if ui.toggle_switch(14.0, enabled).changed() {
                    let message = ControlClientMessage::UpdateEnabledDebugSystem(
                        system_name.clone(),
                        *enabled,
                    );
                    message_sender.tx.unbounded_send(message).unwrap();
                };
            });
        }
    });
}
