use std::net::{Ipv4Addr, SocketAddrV4};

use re_control_comms::{protocol::CONTROL_PORT, viewer::{ControlViewer, ControlViewerHandle}};
use re_viewer::external::{
    egui, re_ui,
    re_viewer_context::{
        SystemExecutionOutput, ViewClass, ViewClassLayoutPriority, ViewClassRegistryError,
        ViewQuery, ViewSpawnHeuristics, ViewState, ViewStateExt, ViewSystemExecutionError,
        ViewSystemRegistrator, ViewerContext,
    },
};
use rerun::external::re_types::ViewClassIdentifier;

use crate::{
    control::{CameraState, DebugEnabledSystemsView},
    ui::debug_systems::debug_enabled_systems_ui,
};

// #[derive(Default)]
struct ControlViewState {
    handle: ControlViewerHandle,
    pub debug_enabled_systems_view: DebugEnabledSystemsView,
    pub camera_state: CameraState,
}

impl Default for ControlViewState {
    fn default() -> Self {
        let socket_addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), CONTROL_PORT);
        let handle = ControlViewer::from(socket_addr).run();
        Self {
            handle,
            debug_enabled_systems_view: Default::default(),
            camera_state: Default::default(),
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
        &re_ui::icons::APPLICATION
    }

    fn help_markdown(&self, _egui_ctx: &re_viewer::external::egui::Context) -> String {
        "A view to control the robot".to_owned()
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
        debug_enabled_systems_ui(ui, state.debug_enabled_systems_view, handle);
        Ok(())
    }

    fn selection_ui(
        &self,
        _ctx: &ViewerContext<'_>,
        _ui: &mut egui::Ui,
        _state: &mut dyn ViewState,
        _space_origin: &rerun::EntityPath,
        _view_id: re_viewer::external::re_viewer_context::ViewId,
    ) -> Result<(), ViewSystemExecutionError> {
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
