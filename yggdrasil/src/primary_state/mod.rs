use crate::{
    filter::button::ChestButton,
    game_controller::GameControllerConfig,
    nao::manager::{NaoManager, Priority},
    prelude::*,
};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};
use std::time::Duration;

use bifrost::communication::{GameControllerMessage, GameState};
use nidhogg::types::color;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PrimaryStateConfig {
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub chest_blink_interval: Duration,
}

/// A module providing information about the primary state of the robot. These
/// states include: "Unstiff", "Initial", "Ready", "Set", "Playing",
/// "Penalized", "Finished" and "Calibration".
///
/// This module provides the following resources to the application:
/// - [`PrimaryState`]
pub struct PrimaryStateModule;

impl Module for PrimaryStateModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_resource(Resource::new(PrimaryState::Unstiff))?
            .add_system(update_primary_state.after(crate::filter::button::button_filter)))
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PrimaryState {
    /// State in which all joints are unstiffened and the robot does not move
    Unstiff,
    /// State at the start of the match where the robots stand up
    Initial,
    /// State in which robots walk to their legal positions
    Ready,
    /// State in which the robots wait for a kick-off or penalty
    Set,
    /// State in which the robots are playing soccer
    Playing,
    /// State when the robot has been penalized. Robot may not move except for
    /// standing up
    Penalized,
    /// State of the robot when a half is finished
    Finished,
    /// State the indicates the robot is performing automatic callibration
    Calibration,
}

impl PrimaryState {
    /// Tell whether the robot should walk in this state.
    pub fn should_walk(&self) -> bool {
        // !matches!(
        //     self,
        //     Self::Unstiff | Self::Penalized | Self::Finished | Self::Calibration
        // )
        true
    }
}

// TODO: Replace with player number from pregame config.
const PLAYER_NUM: u8 = 3;

fn is_penalized(
    game_controller_message: Option<&GameControllerMessage>,
    team_number: u8,
    player_number: u8,
) -> bool {
    game_controller_message.is_some_and(|game_controller_message| {
        game_controller_message
            .team(team_number)
            .map(|team| team.is_penalized(player_number))
            .unwrap_or(false)
    })
}

#[system]
pub fn update_primary_state(
    primary_state: &mut PrimaryState,
    game_controller_message: &Option<GameControllerMessage>,
    nao_manager: &mut NaoManager,
    chest_button: &ChestButton,
    config: &PrimaryStateConfig,
    game_controller_config: &GameControllerConfig,
) -> Result<()> {
    use PrimaryState as PS;

    // TODO: add penalized state
    // We need the robot's id and check the `RobotInfo` array in the game-controller message, to
    // see if this robot has received a penalty.
    let next_primary_state = if is_penalized(
        game_controller_message.as_ref(),
        game_controller_config.team_number,
        PLAYER_NUM,
    ) {
        PrimaryState::Penalized
    } else {
        match game_controller_message {
            Some(message) => match message {
                GameControllerMessage {
                    state: GameState::Initial,
                    ..
                } => PrimaryState::Initial,
                GameControllerMessage {
                    state: GameState::Ready,
                    ..
                } => PrimaryState::Ready,
                GameControllerMessage {
                    state: GameState::Set,
                    ..
                } => PrimaryState::Set,
                GameControllerMessage {
                    state: GameState::Playing,
                    ..
                } => PrimaryState::Playing,
                GameControllerMessage {
                    state: GameState::Finished,
                    ..
                } => PrimaryState::Finished,
            },
            None if chest_button.state.is_tapped() => PrimaryState::Initial,
            None => *primary_state,
        }
    };

    match next_primary_state {
        PS::Unstiff => nao_manager.set_chest_blink_led(
            color::f32::BLUE,
            config.chest_blink_interval,
            Priority::Critical,
        ),
        PS::Initial => nao_manager.set_chest_led(color::f32::EMPTY, Priority::Critical),
        PS::Ready => nao_manager.set_chest_led(color::f32::BLUE, Priority::Critical),
        PS::Set => nao_manager.set_chest_led(color::f32::YELLOW, Priority::Critical),
        PS::Playing => nao_manager.set_chest_led(color::f32::GREEN, Priority::Critical),
        PS::Penalized => nao_manager.set_chest_led(color::f32::RED, Priority::Critical),
        PS::Finished => nao_manager.set_chest_led(color::f32::EMPTY, Priority::Critical),
        PS::Calibration => nao_manager.set_chest_led(color::f32::PURPLE, Priority::Critical),
    };

    *primary_state = next_primary_state;

    Ok(())
}
