use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use bifrost::communication::{GameControllerMessage, GameState};
use re_control_comms::{protocol::ViewerMessage, viewer::ControlViewerHandle};
use rerun::external::egui;

use crate::re_control_view::ControlViewerData;

use super::view_section;

const PACKET_NUMBER: u8 = 0;
const PLAYERS_PER_TEAM: u8 = 1;
const KICKING_TEAM: u8 = 8;
const SECONDARY_TIME: Duration = Duration::from_secs(0);

pub fn game_controller_ui(
    ui: &mut egui::Ui,
    _viewer_data: Arc<RwLock<ControlViewerData>>,
    handle: &ControlViewerHandle,
) {
    view_section(ui, "Game Controller".to_string(), |ui| {
        game_controller_gird(ui, handle);
    });
}

fn game_controller_gird(ui: &mut egui::Ui, handle: &ControlViewerHandle) {
    ui.label("Test");
    egui::Grid::new("game controller")
        .num_columns(2)
        .spacing([20.0, 4.0])
        .show(ui, |ui| {
            state_buttons(ui, handle);
            ui.end_row();
        });
}

fn state_buttons(ui: &mut egui::Ui, handle: &ControlViewerHandle) {
    state_button(ui, handle, "Initial", GameState::Initial);
    state_button(ui, handle, "Standy", GameState::Standby);
    state_button(ui, handle, "Ready", GameState::Ready);
    state_button(ui, handle, "Set", GameState::Set);
    state_button(ui, handle, "Playing", GameState::Playing);
    state_button(ui, handle, "Playing", GameState::Finished);
}

fn state_button(
    ui: &mut egui::Ui,
    handle: &ControlViewerHandle,
    button_name: &str,
    state: GameState,
) {
    if ui.button(button_name).clicked() {
        let message = GameControllerMessage::create(
            PACKET_NUMBER,
            PLAYERS_PER_TEAM,
            state,
            KICKING_TEAM,
            SECONDARY_TIME,
        );
        let message = ViewerMessage::FakeGameControllerMessage { message };

        if let Err(error) = handle.send(message) {
            tracing::error!(?error, "Failed to send message");
        }
    }
}
