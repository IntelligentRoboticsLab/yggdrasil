use std::{
    net::SocketAddrV4,
    sync::{Arc, RwLock},
};

use bifrost::communication::Penalty;
use re_control_comms::{
    protocol::{
        CONTROL_PORT, RobotMessage,
        game_controller::{Player, RobotGameController},
    },
    viewer::ControlViewer,
};
use rerun::external::{
    egui::{self, Color32},
    re_types::ViewClassIdentifier,
    re_ui::{self},
    re_viewer_context::{
        self, SystemExecutionOutput, ViewClass, ViewClassLayoutPriority, ViewClassRegistryError,
        ViewQuery, ViewSpawnHeuristics, ViewState, ViewStateExt, ViewSystemExecutionError,
        ViewSystemRegistrator, ViewerContext,
    },
};

use crate::{
    connection::{ConnectionState, ROBOT_ADDRESS_ENV_KEY, ip_from_env},
    state::{HandleState, SharedHandleState},
    ui::{
        extra_title_bar_connection_ui,
        game_controller::{GameControllerState, game_controller_ui},
        selection_ui,
    },
};

pub(crate) struct GameControllerViewerData {
    pub connected_player: Option<Player>,
    pub penalty_type: Penalty,
    pub game_controller_state: GameControllerState,
}

impl Default for GameControllerViewerData {
    fn default() -> Self {
        Self {
            connected_player: Default::default(),
            penalty_type: Penalty::Manual,
            game_controller_state: Default::default(),
        }
    }
}

struct GameControllerViewState {
    pub connection: ConnectionState,
    pub data: Arc<RwLock<GameControllerViewerData>>,
}

impl Default for GameControllerViewState {
    fn default() -> Self {
        let ip_addr = ip_from_env(ROBOT_ADDRESS_ENV_KEY);

        let socket_addr = SocketAddrV4::new(ip_addr, CONTROL_PORT);
        let control_viewer = ControlViewer::from(socket_addr);

        let data = Arc::new(RwLock::new(GameControllerViewerData::default()));
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
            data,
        }
    }
}

impl ViewState for GameControllerViewState {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Default)]
pub struct GameControllerView;

impl ViewClass for GameControllerView {
    fn identifier() -> ViewClassIdentifier
    where
        Self: Sized,
    {
        "GameControllerView".into()
    }

    fn display_name(&self) -> &'static str {
        "Game Controller"
    }

    fn icon(&self) -> &'static re_ui::Icon {
        &re_ui::icons::APPLICATION
    }

    fn on_register(
        &self,
        _system_registry: &mut ViewSystemRegistrator<'_>,
    ) -> Result<(), ViewClassRegistryError> {
        Ok(())
    }

    fn new_state(&self) -> Box<dyn ViewState> {
        Box::<GameControllerViewState>::default()
    }

    fn layout_priority(&self) -> ViewClassLayoutPriority {
        Default::default()
    }

    fn ui(
        &self,
        _ctx: &ViewerContext<'_>,
        ui: &mut egui::Ui,
        state: &mut dyn ViewState,
        _query: &ViewQuery<'_>,
        _system_output: SystemExecutionOutput,
    ) -> Result<(), ViewSystemExecutionError> {
        let state = state.downcast_mut::<GameControllerViewState>()?;

        let handle = &state.connection.handle;

        game_controller_ui(ui, Arc::clone(&state.data), handle);

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
        let state = state.downcast_mut::<GameControllerViewState>()?;

        ui.add_space(5.0);
        ui.label(
            egui::RichText::new("Connection")
                .color(Color32::WHITE)
                .size(16.0),
        );

        selection_ui::connection_selection_ui(ui, &mut state.connection, Arc::clone(&state.data));

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
        let state = state.downcast_mut::<GameControllerViewState>()?;

        extra_title_bar_connection_ui(ui, &state.connection);

        Ok(())
    }

    fn help(&self, _egui_ctx: &egui::Context) -> re_ui::Help {
        re_ui::Help::new("Mock GameController messages to connected robot")
    }

    fn spawn_heuristics(
        &self,
        _ctx: &ViewerContext<'_>,
        _suggested_filter: &rerun::external::re_log_types::ResolvedEntityPathFilter,
    ) -> ViewSpawnHeuristics {
        ViewSpawnHeuristics::empty()
    }
}

impl HandleState for GameControllerViewerData {
    fn handle_message(&mut self, message: &RobotMessage) {
        if let RobotMessage::RobotGameController(message) = message {
            match message {
                RobotGameController::GameControllerMessage { message } => {
                    self.game_controller_state.update_message(Some(*message));
                }
                RobotGameController::GameControllerMessageInit { team_number } => {
                    self.game_controller_state.init_state(*team_number);
                }
                RobotGameController::PlayerInfo { player } => {
                    self.connected_player = Some(*player);
                }
            }
        }
    }
}
