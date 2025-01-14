use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::{Arc, RwLock},
};

use heimdall::CameraPosition;
use re_control_comms::{
    debug_system::DebugEnabledSystems,
    protocol::{RobotMessage, CONTROL_PORT},
    viewer::{ControlViewer, ControlViewerHandle},
};
use re_viewer::external::{
    egui,
    re_ui::{self, Icon},
    re_viewer_context::{
        SystemExecutionOutput, ViewClass, ViewClassLayoutPriority, ViewClassRegistryError,
        ViewQuery, ViewSpawnHeuristics, ViewState, ViewStateExt, ViewSystemExecutionError,
        ViewSystemRegistrator, ViewerContext,
    },
};
use rerun::external::re_types::ViewClassIdentifier;

use crate::ui::{
    camera_calibration::{camera_calibration_ui, CameraState},
    debug_systems::{debug_enabled_systems_ui, DebugEnabledState},
    resource::{resource_ui, ResourcesState},
};

const CONTROL_PANEL_VIEW: Icon = Icon::new(
    "../data/icons/robot.png",
    include_bytes!("../data/icons/robot.png"),
);

/// The data of the [`ControlViewState`]. This is all data that is available/saved
/// for the [`ControlView`] (which is the custom [`ViewClass`]).
#[derive(Default)]
pub struct ControlViewerData {
    pub resources_state: ResourcesState,
    pub debug_enabled_state: DebugEnabledState,
    pub camera_state: CameraState,
}

#[derive(Clone, Debug, Default, PartialEq)]
enum ControlViewerSection {
    Resources,
    #[default]
    DebugEnabledSystems,
    CameraCalibration,
}

// The state of the custom `ViewClass`. It consists of:
// - `ControlViewerHandle`, used for communication between viewer and robot
// - `ControlViewerData`, which consists of all data of the custom `ViewClass`
struct ControlViewState {
    handle: ControlViewerHandle,
    control_view_section: ControlViewerSection,
    data: Arc<RwLock<ControlViewerData>>,
}

impl Default for ControlViewState {
    fn default() -> Self {
        let socket_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), CONTROL_PORT);
        let control_viewer = ControlViewer::from(socket_addr);

        let data = Arc::new(RwLock::new(ControlViewerData::default()));
        let handler_data = Arc::clone(&data);

        // Add a handler for the `ControlViewer` before it runs. This is to
        // make sure we do not miss any message send at the beginning of a
        // connection
        control_viewer
            .add_handler(Box::new(move |msg: &RobotMessage| {
                handle_message(msg, Arc::clone(&handler_data))
            }))
            .expect("Failed to add handler");

        let handle = control_viewer.run();
        Self {
            handle,
            control_view_section: ControlViewerSection::default(),
            data,
        }
    }
}

impl ViewState for ControlViewState {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Default)]
pub struct ControlView;

impl ViewClass for ControlView {
    fn identifier() -> ViewClassIdentifier {
        "ControlView".into()
    }

    fn display_name(&self) -> &'static str {
        "Control viewer"
    }

    fn icon(&self) -> &'static re_ui::Icon {
        &CONTROL_PANEL_VIEW
    }

    fn help_markdown(&self, _egui_ctx: &re_viewer::external::egui::Context) -> String {
        format!(
            "# Control View

A view to control the robot",
        )
    }

    fn on_register(
        &self,
        _system_registry: &mut ViewSystemRegistrator<'_>,
    ) -> Result<(), ViewClassRegistryError> {
        Ok(())
    }

    fn new_state(&self) -> Box<dyn ViewState> {
        Box::<ControlViewState>::default()
    }

    fn layout_priority(&self) -> ViewClassLayoutPriority {
        Default::default()
    }

    fn spawn_heuristics(&self, _ctx: &ViewerContext<'_>) -> ViewSpawnHeuristics {
        ViewSpawnHeuristics::root()
    }

    fn ui(
        &self,
        _ctx: &ViewerContext<'_>,
        ui: &mut egui::Ui,
        state: &mut dyn ViewState,
        _query: &ViewQuery<'_>,
        _system_output: SystemExecutionOutput,
    ) -> Result<(), ViewSystemExecutionError> {
        let state = state.downcast_mut::<ControlViewState>()?;

        // Show different ui based on the chosen view section
        match state.control_view_section {
            ControlViewerSection::Resources => {
                // Resource section
                resource_ui(ui, Arc::clone(&state.data), &state.handle);
            }
            ControlViewerSection::DebugEnabledSystems => {
                // Debug enabled/disabled systems section
                debug_enabled_systems_ui(ui, Arc::clone(&state.data), &state.handle);
            }
            ControlViewerSection::CameraCalibration => {
                // Camera calibration section
                camera_calibration_ui(ui, Arc::clone(&state.data), &state.handle);
            }
        }

        Ok(())
    }

    fn selection_ui(
        &self,
        _ctx: &ViewerContext<'_>,
        ui: &mut egui::Ui,
        state: &mut dyn ViewState,
        _space_origin: &rerun::EntityPath,
        _view_id: re_viewer::external::re_viewer_context::ViewId,
    ) -> Result<(), ViewSystemExecutionError> {
        let state = state.downcast_mut::<ControlViewState>()?;

        ui.label("Viewer section selection");

        let selected = &mut state.control_view_section;
        egui::ComboBox::from_id_salt("control viewer section selection")
            .selected_text(format!("{:?}", selected))
            .show_ui(ui, |ui| {
                ui.selectable_value(selected, ControlViewerSection::Resources, "Resources");
                ui.selectable_value(
                    selected,
                    ControlViewerSection::DebugEnabledSystems,
                    "DebugEnabledSystems",
                );
                ui.selectable_value(
                    selected,
                    ControlViewerSection::CameraCalibration,
                    "CameraCalibration",
                );
            });

        Ok(())
    }

    fn extra_title_bar_ui(
        &self,
        _ctx: &ViewerContext<'_>,
        _ui: &mut egui::Ui,
        _state: &mut dyn ViewState,
        _space_origin: &rerun::EntityPath,
        _view_id: re_viewer::external::re_viewer_context::ViewId,
    ) -> Result<(), ViewSystemExecutionError> {
        Ok(())
    }
}

fn handle_message(message: &RobotMessage, data: Arc<RwLock<ControlViewerData>>) {
    match message {
        RobotMessage::DebugEnabledSystems(enabled_systems) => {
            data.write()
                .expect("Failed to lock viewer data")
                .debug_enabled_state
                .update(DebugEnabledSystems::from(enabled_systems.clone()));
        }
        RobotMessage::Resources(_resources) => {
            tracing::warn!("Got a resource update but is unhandled")
        }
        RobotMessage::CameraExtrinsic {
            camera_position,
            extrinsic_rotation,
        } => {
            let mut locked_data = data.write().expect("Failed to lock viewer data");
            let camera_config = &mut locked_data.camera_state;

            let camera = match camera_position {
                CameraPosition::Top => &mut camera_config.config.top,
                CameraPosition::Bottom => &mut camera_config.config.bottom,
            };

            camera_config.current_position = *camera_position;
            camera.extrinsic_rotation.new_state(*extrinsic_rotation);
        }
    }
}
