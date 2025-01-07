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
use re_ui::{Icon, UiExt};
use re_viewer::external::{
    eframe,
    egui::{self, Align2, ScrollArea, Vec2},
};

use crate::{
    resource::RobotResources,
    ui::{
        camera_calibration::camera_calibration_ui, debug_systems::debug_enabled_systems_ui,
        resource::resource_ui, style::FrameStyleMap, PANEL_TOP_PADDING, SIDE_PANEL_WIDTH,
    },
};

pub const CONTROL_PANEL_VIEW: Icon = Icon::new(
    "../data/icons/robot.png",
    include_bytes!("../data/icons/robot.png"),
);

pub const CONTROL_PANEL_VIEW_OFF: Icon = Icon::new(
    "../data/icons/robot_off.png",
    include_bytes!("../data/icons/robot_off.png"),
);

/// Position of the toggle button when the side bar is visible.
const SIDE_BAR_TOGGLE_BUTTON_POSITION_VISIBLE: Vec2 = Vec2::new(-10., 5.);

/// Position of the toggle button when the side bar is hidden.
const SIDE_BAR_TOGGLE_BUTTON_POSITION_HIDDEN: Vec2 = Vec2::new(-251., 4.);

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
    pub side_bar_toggled: bool,
}

impl eframe::App for Control {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Store viewer state on disk
        self.app.save(storage);
    }

    /// Called whenever we need repainting, which could be 60 Hz.
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        egui::Area::new(egui::Id::new("re_control")).show(ctx, |ui| {
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
            side_bar_toggled: true,
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        self.toggle_button_overlay(ui);

        if !self.side_bar_toggled {
            ui.set_width(30.);
            ui.shrink_width_to_current();
            return;
        }

        egui::SidePanel::right("control")
            .default_width(SIDE_PANEL_WIDTH)
            .show(ui.ctx(), |ui| {
                ui.add_space(PANEL_TOP_PADDING);

                // Title of the side panel
                ui.vertical_centered(|ui| {
                    ui.strong("Control panel");
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
            });
    }

    fn toggle_button_overlay(&mut self, ui: &mut egui::Ui) {
        let (icon, position) = if self.side_bar_toggled {
            (
                &CONTROL_PANEL_VIEW_OFF,
                SIDE_BAR_TOGGLE_BUTTON_POSITION_VISIBLE,
            )
        } else {
            (&CONTROL_PANEL_VIEW, SIDE_BAR_TOGGLE_BUTTON_POSITION_HIDDEN)
        };

        egui::Area::new("control_panel_toggle_overlay".into())
            .anchor(Align2::RIGHT_TOP, position)
            .show(ui.ctx(), |ui| {
                ui.medium_icon_toggle_button(icon, &mut self.side_bar_toggled)
                    .on_hover_text("Toggle control panel")
            });
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
