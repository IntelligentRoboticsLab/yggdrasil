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
    connection::{
        protocol::{RobotMessage, ViewerMessage},
        viewer::ControlViewerHandle,
    },
    debug_system::DebugEnabledSystems,
    resource::RobotResources,
    ui::{
        add_editable_resource, debug_resources_ui,
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

impl From<DebugEnabledSystems> for DebugEnabledSystemsView {
    fn from(debug_enabled_systems: DebugEnabledSystems) -> Self {
        let mut key_sequence: Vec<_> = debug_enabled_systems.systems.keys().cloned().collect();
        key_sequence.sort();
        Self {
            debug_enabled_systems,
            key_sequence,
        }
    }
}

pub struct Control {
    app: re_viewer::App,
    states: Arc<RwLock<ControlStates>>,
    handle: ControlViewerHandle<ViewerMessage, RobotMessage>,
    frame_styles: FrameStyleMap,
}

impl eframe::App for Control {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Store viewer state on disk
        self.app.save(storage);
    }

    /// Called whenever we need repainting, which could be 60 Hz.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Feels weird to put the `handle_message` in a function
        // that is executed when ui needs repainting
        // if let Some(mut message_receiver) = self.message_receiver.take() {
        //     self.message_receiver = match handle_message(&mut message_receiver, &mut self.states) {
        //         HandleMessageStatus::Stopped => None,
        //         HandleMessageStatus::Continue => Some(message_receiver),
        //     }
        // }

        // First add our panel(s):
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
    pub fn new(
        app: re_viewer::App,
        handle: ControlViewerHandle<ViewerMessage, RobotMessage>,
    ) -> Self {
        let mut control = Control {
            app,
            states: Arc::new(RwLock::new(ControlStates::default())),
            handle,
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
                        .expect("Failed to read lock states")
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

        //
        // Resource section
        //

        // Sort the names to keep the resources in a fixed order
        let mut resource_names: Vec<_>;
        {
            let states = self.states.read().expect("Failed to lock states");
            resource_names = states.robot_resources.0.keys().cloned().collect();
        }
        resource_names.sort();

        for name in resource_names {
            let mut states = self.states.write().expect("Failed to lock states");
            if let Some(data) = states.robot_resources.0.get_mut(&name) {
                let followup_action = add_editable_resource(
                    ui,
                    &name,
                    data,
                    Arc::clone(&self.states),
                    self.frame_styles
                        .get_or_default("override_button".to_string()),
                );
                if let Some(action) = followup_action {
                    self.handle.send(action).unwrap();
                }
            }
        }

        ui.separator();

        ui.vertical_centered(|ui| {
            ui.strong("Systems debug on/off");
        });

        // Debug enabled/disabled systems sections

        ui.horizontal(|ui| debug_resources_ui(ui, Arc::clone(&self.states), &self.handle));

    // fn listen_for_robot_messages(
    //     reader: ReadHalf<TcpStream>,
    // ) -> ControlReceiver<ControlRobotMessage> {
    //     let (reader_tx, reader_rx) = mpsc::unbounded::<ControlRobotMessage>();
    //     tokio::spawn(async move {
    //         receive_messages(reader, reader_tx).await;
    //     });

    //     ControlReceiver { rx: reader_rx }
    // }

    // fn setup_send_messages_to_robot(
    //     writer: WriteHalf<TcpStream>,
    // ) -> ControlSender<ControlViewerMessage> {
    //     let (writer_tx, writer_rx) = mpsc::unbounded::<ControlViewerMessage>();
    //     tokio::spawn(async move {
    //         send_messages(writer, writer_rx).await;
    //     });
    //     ControlSender { tx: writer_tx }
    // }
    }
}

// Temp handler
fn handle_message(message: &RobotMessage, states: Arc<RwLock<ControlStates>>) {
    match message {
        RobotMessage::Disconnect => {
            tracing::info!("Robot disconnected")
        }
        RobotMessage::DebugEnabledSystems(enabled_systems) => {
            tracing::info!("Update debug enabled systems");
            // Probably change `.into()` to a `.update()` function
            states
                .write()
                .expect("Failed to write lock control states")
                .debug_enabled_systems_view = enabled_systems.clone().into()
        }
        RobotMessage::Resources(resources) => {
            tracing::info!("Got a resource update")
        }
    }
}
