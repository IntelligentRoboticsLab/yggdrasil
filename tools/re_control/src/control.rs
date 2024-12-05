use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    time::Instant,
};

use heimdall::CameraPosition;
use nalgebra::Vector3;
use re_control_comms::{
    debug_system::DebugEnabledSystems,
    protocol::RobotMessage,
    viewer::{ControlViewer, ControlViewerHandle},
};
use re_viewer::external::{
    eframe,
    egui::{self, ScrollArea},
};

use crate::{
    resource::RobotResources,
    ui::{
        camera_calibration::camera_calibration_ui, debug_systems::debug_enabled_systems_ui,
        resource::resource_ui, style::FrameStyleMap, PANEL_TOP_PADDING, SIDE_PANEL_WIDTH,
    },
};

#[derive(Default)]
pub struct ControlStates {
    pub robot_resources: RobotResources,
    pub focused_resources: HashMap<String, bool>,
    pub last_resource_update: Option<Instant>,
    pub debug_enabled_systems_view: DebugEnabledSystemsView,
    pub camera_state: CameraState,
}

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
    pub extrinsic_rotation: State<Vector3<f32>>,
}

#[derive(Default)]
pub struct State<T> {
    current: T,
    original: T,
}

impl<T> State<T>
where
    T: Clone,
{
    pub fn current(&self) -> &T {
        &self.current
    }

    pub fn current_mut(&mut self) -> &mut T {
        &mut self.current
    }

    pub fn new_state(&mut self, state: T) {
        self.current = state.clone();
        self.original = state;
    }

    pub fn restore_from_original(&mut self) {
        self.current = self.original.clone();
    }
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
            .default_width(SIDE_PANEL_WIDTH)
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    self.ui(ui);
                });
            });

        self.app.update(ctx, frame);
    }
}

impl Control {
    pub fn new(app: re_viewer::App, control_viewer: ControlViewer) -> Self {
        let states = Arc::new(RwLock::new(ControlStates::default()));
        let handler_states = Arc::clone(&states);

        // Add a handler for the `ControlViewer` before it runs. This is to
        // make sure we do not miss any message send at the beginning of a
        // connection
        control_viewer
            .add_handler(Box::new(move |msg: &RobotMessage| {
                handle_message(msg, Arc::clone(&handler_states))
            }))
            .expect("Failed to add handler");

        // Start up the `ControlViewer` which will try to connection to
        // a `ControlApp`
        let handle = control_viewer.run();

        Control {
            app,
            states: Arc::clone(&states),
            handle,
            frame_styles: FrameStyleMap::default(),
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.style_mut().spacing.item_spacing.y = 0.;
        ui.add_space(4.);
        ui.horizontal(|ui| {
            let mut selected = true;
            if ui
                .medium_icon_toggle_button(&re_ui::icons::RIGHT_PANEL_TOGGLE, &mut selected)
                .on_hover_text(format!("Toggle selection view",))
                .clicked()
            {}

            ui.strong("Control panel");
        });
        ui.separator();

        ui.horizontal(|ui| {
            ui.add_space(ui.available_width());
        });

        // Resource section
        resource_ui(
            ui,
            Arc::clone(&self.states),
            &self.handle,
            &self.frame_styles,
        );

        // Debug enabled/disabled systems section
        debug_enabled_systems_ui(ui, Arc::clone(&self.states), &self.handle);

        // Camera calibration section
        camera_calibration_ui(ui, Arc::clone(&self.states), &self.handle);
    }
}

fn handle_message(message: &RobotMessage, states: Arc<RwLock<ControlStates>>) {
    match message {
        RobotMessage::DebugEnabledSystems(enabled_systems) => {
            states
                .write()
                .expect("Failed to lock states")
                .debug_enabled_systems_view
                .update(DebugEnabledSystems::from(enabled_systems.clone()));
        }
        RobotMessage::Resources(_resources) => {
            tracing::warn!("Got a resource update but is unhandled")
        }
        RobotMessage::CameraExtrinsic {
            camera_position,
            extrinsic_rotation,
        } => {
            let mut states = states.write().expect("Failed to lock states");

            let camera_config = &mut states.camera_state;

            let camera = match camera_position {
                CameraPosition::Top => &mut camera_config.config.top,
                CameraPosition::Bottom => &mut camera_config.config.bottom,
            };

            camera_config.current_position = *camera_position;
            camera.extrinsic_rotation.new_state(*extrinsic_rotation);
        }
    }
}
