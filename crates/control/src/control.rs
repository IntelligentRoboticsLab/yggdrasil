use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Instant,
};

use re_viewer::external::{
    eframe,
    egui::{self, ScrollArea},
};

use crate::{
    connection::{protocol::RobotMessage, viewer::ControlViewerHandle},
    debug_system::DebugEnabledSystems,
    handle_message::handle_message,
    resource::RobotResources,
    ui::{
        debug_resources_ui, resource_ui,
        style::{FrameStyleMap, LAST_UPDATE_COLOR},
    },
};

#[derive(Default)]
pub struct ControlStates {
    pub robot_resources: RobotResources,
    pub focused_resources: HashMap<String, bool>,
    pub last_resource_update: Option<Instant>,
    pub debug_enabled_systems_view: DebugEnabledSystemsView,
}

#[derive(Default)]
pub struct DebugEnabledSystemsView {
    pub debug_enabled_systems: DebugEnabledSystems,
    pub key_sequence: Vec<String>,
}

impl DebugEnabledSystemsView {
    pub fn update(&mut self, debug_enabled_systems: DebugEnabledSystems) {
        let mut key_sequence: Vec<_> = debug_enabled_systems.systems.keys().cloned().collect();
        key_sequence.sort();
        self.debug_enabled_systems = debug_enabled_systems;
        self.key_sequence = key_sequence;
    }
}

pub struct Control {
    app: re_viewer::App,
    states: Arc<RwLock<ControlStates>>,
    handle: ControlViewerHandle,
    frame_styles: FrameStyleMap,
}

impl eframe::App for Control {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Store viewer state on disk
        self.app.save(storage);
    }

    /// Called whenever we need repainting, which could be 60 Hz.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::SidePanel::right("Resource manipulation")
            .default_width(400.0)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    self.ui(ui);
                });
            });

        self.app.update(ctx, frame);
    }
}

impl Control {
    pub fn new(app: re_viewer::App, handle: ControlViewerHandle) -> Self {
        let mut control = Control {
            app,
            states: Arc::new(RwLock::new(ControlStates::default())),
            handle: handle.clone(),
            frame_styles: FrameStyleMap::default(),
        };

        let states = Arc::clone(&control.states);

        control
            .handle
            .add_handler(move |msg: &RobotMessage| handle_message(msg, Arc::clone(&states)))
            .expect("Failed to add handler");

        control
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        // Title of the side panel
        ui.vertical_centered(|ui| {
            ui.strong("Control panel");
        });
        ui.separator();

        ui.horizontal(|ui| {
            ui.add_space(ui.available_width());

            // Shows the last resource update in milliseconds
            ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                let last_resource_update = {
                    match self
                        .states
                        .read()
                        .expect("Failed to lock states")
                        .last_resource_update
                    {
                        Some(time) => time.elapsed().as_millis(),
                        None => 0,
                    }
                };

                ui.label(
                    egui::RichText::new(format!(
                        "Last updated: {:>4} ms ago",
                        last_resource_update
                    ))
                    .monospace()
                    .color(LAST_UPDATE_COLOR),
                );
            });
        });

        ui.add_space(10.0);

        // Resource section
        resource_ui(
            ui,
            Arc::clone(&self.states),
            &self.handle,
            &self.frame_styles,
        );

        ui.separator();

        ui.vertical_centered(|ui| {
            ui.strong("Systems debug on/off");
        });

        // Debug enabled/disabled systems sections
        ui.horizontal(|ui| debug_resources_ui(ui, Arc::clone(&self.states), &self.handle));
    }
}
