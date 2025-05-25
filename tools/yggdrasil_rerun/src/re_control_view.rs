use std::{
    net::SocketAddrV4,
    sync::{Arc, RwLock},
};

use heimdall::CameraPosition;
use yggdrasil_rerun_comms::{
    debug_system::DebugEnabledSystems,
    protocol::{CONTROL_PORT, RobotMessage, control::RobotControlMessage},
    viewer::ControlViewer,
};
use rerun::external::{
    ecolor::Color32,
    egui,
    re_types::ViewClassIdentifier,
    re_ui::{self, Help, Icon},
    re_viewer_context::{
        self, SystemExecutionOutput, ViewClass, ViewClassLayoutPriority, ViewClassRegistryError,
        ViewQuery, ViewSpawnHeuristics, ViewState, ViewStateExt, ViewSystemExecutionError,
        ViewSystemRegistrator, ViewerContext,
    },
};
use strum::{EnumIter, IntoEnumIterator};

use crate::{
    connection::{ConnectionState, ROBOT_ADDRESS_ENV_KEY, ip_from_env},
    state::{HandleState, SharedHandleState},
    ui::{
        camera_calibration::{CameraState, camera_calibration_ui},
        debug_systems::{DebugEnabledState, debug_enabled_systems_ui},
        extra_title_bar_connection_ui,
        field_color::{FieldColorState, field_color_ui},
        resource::{ResourcesState, resource_ui},
        selection_ui,
        visual_referee::visual_referee_ui,
    },
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
    pub field_color: FieldColorState,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, EnumIter)]
enum ControlViewerSection {
    CameraCalibration,
    #[default]
    DebugEnabledSystems,
    FieldColor,
    Resources,
    VisualReferee,
}

/// The state of the custom `ViewClass`. It consists of:
/// - connection: `ControlViewerHandle`, used for communication between viewer and robot
/// - data: `ControlViewerData`, which is the data from the connected robot
pub struct ControlViewState {
    pub connection: ConnectionState,
    control_view_section: ControlViewerSection,
    pub data: Arc<RwLock<ControlViewerData>>,
}

impl Default for ControlViewState {
    fn default() -> Self {
        let ip_addr = ip_from_env(ROBOT_ADDRESS_ENV_KEY);

        let socket_addr = SocketAddrV4::new(ip_addr, CONTROL_PORT);
        let control_viewer = ControlViewer::from(socket_addr);

        let data = Arc::new(RwLock::new(ControlViewerData::default()));
        let handler_data = Arc::clone(&data);

        // Add a handler for the `ControlViewer` before it runs.
        // This is to make sure we do not miss any message sent at
        // the beginning of a connection.
        control_viewer
            .add_handler(Box::new(move |msg: &RobotMessage| {
                Arc::clone(&handler_data).handle_message(msg)
            }))
            .expect("Failed to add handler");

        let handle = control_viewer.run();

        Self {
            connection: ConnectionState::from_handle(handle),
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

    fn help(&self, _egui_ctx: &egui::Context) -> re_ui::Help {
        Help::new(
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

    fn spawn_heuristics(
        &self,
        _ctx: &ViewerContext<'_>,
        _suggested_filter: &rerun::external::re_log_types::ResolvedEntityPathFilter,
    ) -> ViewSpawnHeuristics {
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

        let handle = &state.connection.handle;

        // Show different ui based on the chosen view section
        match state.control_view_section {
            ControlViewerSection::Resources => {
                // Resource section
                resource_ui(ui, Arc::clone(&state.data), handle);
            }
            ControlViewerSection::DebugEnabledSystems => {
                // Debug enabled/disabled systems section
                debug_enabled_systems_ui(ui, Arc::clone(&state.data), handle);
            }
            ControlViewerSection::CameraCalibration => {
                // Camera calibration section
                camera_calibration_ui(ui, Arc::clone(&state.data), handle);
            }
            ControlViewerSection::FieldColor => {
                field_color_ui(ui, Arc::clone(&state.data), handle);
            }
            ControlViewerSection::VisualReferee => {
                visual_referee_ui(ui, Arc::clone(&state.data), handle);
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
        _view_id: re_viewer_context::ViewId,
    ) -> Result<(), ViewSystemExecutionError> {
        let state = state.downcast_mut::<ControlViewState>()?;

        ui.add_space(5.0);
        ui.label(
            egui::RichText::new("Connection")
                .color(Color32::WHITE)
                .size(16.0),
        );

        selection_ui::connection_selection_ui(ui, &mut state.connection, Arc::clone(&state.data));

        ui.separator();

        ui.label(
            egui::RichText::new("Viewer section selection")
                .color(Color32::WHITE)
                .size(16.0),
        );

        let selected = &mut state.control_view_section;
        egui::ComboBox::from_id_salt("control viewer section selection")
            .selected_text(format!("{:?}", selected))
            .show_ui(ui, |ui| {
                for section in ControlViewerSection::iter() {
                    ui.selectable_value(selected, section, format!("{:?}", section));
                }
            });

        Ok(())
    }

    fn extra_title_bar_ui(
        &self,
        _ctx: &ViewerContext<'_>,
        ui: &mut egui::Ui,
        state: &mut dyn ViewState,
        _space_origin: &rerun::EntityPath,
        _view_id: re_viewer_context::ViewId,
    ) -> Result<(), ViewSystemExecutionError> {
        let state = state.downcast_mut::<ControlViewState>()?;

        extra_title_bar_connection_ui(ui, &state.connection);

        Ok(())
    }
}

impl HandleState for ControlViewerData {
    fn handle_message(&mut self, message: &RobotMessage) {
        if let RobotMessage::RobotControlMessage(message) = message {
            match message {
                RobotControlMessage::DebugEnabledSystems(enabled_systems) => {
                    self.debug_enabled_state
                        .update(DebugEnabledSystems::from(enabled_systems.clone()));
                }
                RobotControlMessage::Resources(_resources) => {
                    tracing::warn!("Got a resource update but is unhandled")
                }
                RobotControlMessage::CameraExtrinsic {
                    camera_position,
                    extrinsic_rotation,
                } => {
                    let camera_config = &mut self.camera_state;

                    let camera = match camera_position {
                        CameraPosition::Top => &mut camera_config.config.top,
                        CameraPosition::Bottom => &mut camera_config.config.bottom,
                    };

                    camera_config.current_position = *camera_position;
                    camera.extrinsic_rotation.new_state(*extrinsic_rotation);
                }
                RobotControlMessage::FieldColor { config } => {
                    self.field_color.config = config.clone();
                }
            }
        }
    }
}
