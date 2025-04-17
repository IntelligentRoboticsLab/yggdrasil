use std::{
    sync::{Arc, RwLock},
    time::Duration,
};

use bifrost::communication::{GameControllerMessage, GameState, Penalty, TeamInfo};
use miette::{Diagnostic, IntoDiagnostic, Result};
use re_control_comms::{
    protocol::{ViewerMessage, game_controller::ViewerGameControllerMessage},
    viewer::ControlViewerHandle,
};
use rerun::external::{
    egui::{self, Color32},
    re_ui::UiExt,
};
use strum::IntoEnumIterator;
use thiserror::Error;

use crate::game_controller_view::GameControllerViewerData;

use super::view_section;

const PENALIZED_TIME: Duration = Duration::from_secs(45);
const MAX_PENALTY_SECONDS: u8 = u8::MAX;

#[derive(Error, Diagnostic, Debug)]
enum GameControllerViewerError {
    #[error("No game controller message exists")]
    EmptyGameControllerMessage,
    #[error("Team number {team_number} is not a team that is playing")]
    #[diagnostic(help(
        "The team number must correspond to one of the playing teams (got {team_number})."
    ))]
    InvalidTeamNumber { team_number: u8 },
    #[error("Player {player_number} is invalid")]
    #[diagnostic(help(
        "The player number must be between 1 and 20, inclusive (got {player_number})."
    ))]
    InvalidPlayerNumber { player_number: u8 },
    #[error("Invalid penalty duration")]
    #[diagnostic(help(
        "Penalty duration must be between 0 and {} seconds",
        MAX_PENALTY_SECONDS
    ))]
    PenaltyDurationTooLong(u64),
}

#[derive(Default)]
pub(crate) struct GameControllerState {
    game_controller: Option<GameControllerMessage>,
}

impl GameControllerState {
    /// Initialize/Reset the internal game controller message of the viewer
    /// with a default [`GameControllerMessage`]. Also set the team number
    /// of the first team in the game controller message.
    pub fn init_state(&mut self, team_number: u8) {
        let mut team = TeamInfo::invisible();
        team.team_number = team_number;

        let message = GameControllerMessage {
            teams: [team, TeamInfo::invisible()],
            ..Default::default()
        };

        self.game_controller = Some(message);
    }

    /// Set/overwrite the internal game controller message of the viewer
    pub fn update_message(&mut self, message: Option<GameControllerMessage>) {
        self.game_controller = message;
    }

    /// Get the game controller message of the viewer
    pub fn message(&self) -> Option<GameControllerMessage> {
        self.game_controller
    }

    fn update_game_state(&mut self, state: GameState) {
        if let Some(message) = &mut self.game_controller {
            message.state = state;
        }
    }

    fn update_penalize_state(
        &mut self,
        team_number: u8,
        player_number: u8,
        penalty: Penalty,
        duration: Duration,
    ) -> Result<()> {
        let penalty_seconds = {
            let seconds = duration.as_secs();
            if seconds > MAX_PENALTY_SECONDS as u64 {
                return Err(GameControllerViewerError::PenaltyDurationTooLong(seconds))
                    .into_diagnostic();
            }
            seconds as u8
        };

        let message = self
            .game_controller
            .as_mut()
            .ok_or(GameControllerViewerError::EmptyGameControllerMessage)?;

        let team = message
            .team_mut(team_number)
            .ok_or(GameControllerViewerError::InvalidTeamNumber { team_number })?;

        let robot = team
            .players
            .get_mut(player_number as usize - 1)
            .ok_or(GameControllerViewerError::InvalidPlayerNumber { player_number })?;

        robot.penalty = penalty;
        robot.secs_till_unpenalised = penalty_seconds;

        Ok(())
    }
}

pub(crate) fn game_controller_ui(
    ui: &mut egui::Ui,
    viewer_data: Arc<RwLock<GameControllerViewerData>>,
    handle: &ControlViewerHandle,
) {
    view_section(ui, "Game Controller".to_string(), |ui| {
        {
            let Ok(locked_data) = &mut viewer_data.write() else {
                ui.vertical_centered_justified(|ui| {
                    ui.warning_label("Not able to access viewer data");
                });
                tracing::warn!("Failed to lock viewer data");
                return;
            };

            if locked_data.game_controller_state.game_controller.is_none() {
                ui.vertical_centered_justified(|ui| {
                    ui.warning_label(
                        "There does not exist a game controller message. Connect to a robot",
                    );
                });
                return;
            }
        }

        game_controller_grid(ui, viewer_data, handle);
    });
}

fn game_controller_grid(
    ui: &mut egui::Ui,
    viewer_data: Arc<RwLock<GameControllerViewerData>>,
    handle: &ControlViewerHandle,
) {
    ui.label(
        egui::RichText::new("Game State")
            .color(Color32::WHITE)
            .size(14.0),
    );
    state_buttons(ui, Arc::clone(&viewer_data), handle);

    ui.separator();
    ui.add_space(3.0);
    ui.label(
        egui::RichText::new("Penalize")
            .color(Color32::WHITE)
            .size(14.0),
    );

    penalize_robot(ui, viewer_data, handle);
}

fn state_buttons(
    ui: &mut egui::Ui,
    viewer_data: Arc<RwLock<GameControllerViewerData>>,
    handle: &ControlViewerHandle,
) {
    ui.horizontal(|ui| {
        for state in GameState::in_order() {
            state_button(
                ui,
                handle,
                Arc::clone(&viewer_data),
                &format!("{:?}", state),
                state,
            );
        }
    });
}

fn state_button(
    ui: &mut egui::Ui,
    handle: &ControlViewerHandle,
    viewer_data: Arc<RwLock<GameControllerViewerData>>,
    button_name: &str,
    next_state: GameState,
) {
    let Ok(locked_data) = &mut viewer_data.write() else {
        ui.vertical_centered_justified(|ui| {
            ui.warning_label("Not able to access viewer data");
        });
        tracing::warn!("Failed to lock viewer data");
        return;
    };

    if ui.button(button_name).clicked() {
        // Update the game controller message saved in the viewer
        locked_data
            .game_controller_state
            .update_game_state(next_state);

        if let Some(message) = locked_data.game_controller_state.message() {
            send_game_controller_message(handle, message);
        }
    }
}

fn penalize_robot(
    ui: &mut egui::Ui,
    viewer_data: Arc<RwLock<GameControllerViewerData>>,
    handle: &ControlViewerHandle,
) {
    let Ok(locked_data) = &mut viewer_data.write() else {
        ui.vertical_centered_justified(|ui| {
            ui.warning_label("Not able to access viewer data");
        });
        tracing::warn!("Failed to lock viewer data");
        return;
    };

    let Some(connected_player) = locked_data.connected_player else {
        ui.vertical_centered_justified(|ui| {
            ui.warning_label("No player is connected");
        });
        return;
    };

    ui.vertical(|ui| {
        ui.add_space(3.0);

        let margin = 7.5;

        ui.horizontal(|ui| {
            ui.set_width(ui.available_width() - 2.0 * margin);
            // Combo box to select the type of penalty
            egui::ComboBox::from_id_salt("Penalty type selection")
                .selected_text(format!("{:?}", locked_data.penalty_type))
                .show_ui(ui, |ui| {
                    for penalty_type in
                        Penalty::iter().filter(|penalty_type| *penalty_type != Penalty::None)
                    {
                        ui.selectable_value(
                            &mut locked_data.penalty_type,
                            penalty_type,
                            format!("{:?}", penalty_type),
                        );
                    }
                });
        });
        // The penalize and unpenalize buttons
        ui.horizontal(|ui| {
            ui.set_width(ui.available_width() - 2.0 * margin); // Adjust width to include margin

            let button_width = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;

            // Button to penalize a robot
            if ui
                .add_sized(
                    [button_width, 25.0],
                    egui::Button::new("Apply Penalty").fill(egui::Color32::from_gray(40)),
                )
                .clicked()
            {
                // Update the game controller message with the penalized robot
                if let Err(error) = locked_data.game_controller_state.update_penalize_state(
                    connected_player.team_number,
                    connected_player.player_number,
                    Penalty::Manual,
                    PENALIZED_TIME,
                ) {
                    tracing::error!(?error, "Failed to penalize robot");
                }
                // Send the current state of the game controller to the robot
                if let Some(message) = locked_data.game_controller_state.message() {
                    send_game_controller_message(handle, message);
                }
            }

            // Button to unpenalize a robot
            if ui
                .add_sized(
                    [button_width, 25.0],
                    egui::Button::new("Unpenalize").fill(egui::Color32::from_gray(40)),
                )
                .clicked()
            {
                // Update the game controller message with the unpenalized robot
                if let Err(error) = locked_data.game_controller_state.update_penalize_state(
                    connected_player.team_number,
                    connected_player.player_number,
                    Penalty::None,
                    PENALIZED_TIME,
                ) {
                    tracing::error!(?error, "Failed to unpenalize robot");
                }
                // Send the current state of the game controller to the robot
                if let Some(message) = locked_data.game_controller_state.message() {
                    send_game_controller_message(handle, message);
                }
            }
        });
    });
}

// Send the saved game controller message from the viewer to the robot
fn send_game_controller_message(
    handle: &ControlViewerHandle,
    game_controller_message: GameControllerMessage,
) {
    // Send game controller message to robot
    let message =
        ViewerMessage::ViewerGameController(ViewerGameControllerMessage::GameControllerMessage {
            message: game_controller_message,
        });

    if let Err(error) = handle.send(message) {
        tracing::error!(?error, "Failed to send message");
    }
}
