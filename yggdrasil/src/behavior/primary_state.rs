use crate::{
    core::{config::showtime::PlayerConfig, whistle::WhistleState},
    nao::manager::{NaoManager, Priority},
    prelude::*,
    sensor::button::{ChestButton, HeadButtons},
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
            .add_system(update_primary_state.after(crate::sensor::button::button_filter)))
    }
}

#[derive(Debug, Clone, PartialEq, Copy, Default)]
pub enum PrimaryState {
    /// State in which all joints are unstiffened and the robot does not move
    #[default]
    Unstiff,
    /// State at the start of the match where the robots stand up.
    /// It's the same state as initial, but robots will not be penalized for a motion in set.
    Standby,
    /// State at the start of the match where the robots stand up
    Initial,
    /// State in which robots walk to their legal positions
    Ready,
    /// State in which the robots wait for a kick-off or penalty
    Set,
    /// State in which the robots are playing soccer, with a bool to keep state after a whistle
    Playing { whistle_in_set: bool },
    /// State when the robot has been penalized. Robot may not move except for
    /// standing up
    Penalized,
    /// State of the robot when a half is finished
    Finished,
    /// State the indicates the robot is performing automatic callibration
    Calibration,
}

fn is_penalized_by_game_controller(
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
    (head_buttons, chest_button): (&HeadButtons, &ChestButton),
    config: &PrimaryStateConfig,
    player_config: &PlayerConfig,
    whistle_state: &WhistleState,
) -> Result<()> {
    use PrimaryState as PS;
    let next_state = next_primary_state(
        primary_state,
        game_controller_message,
        chest_button,
        head_buttons,
        player_config,
        whistle_state,
    );

    match next_state {
        PS::Unstiff => nao_manager.set_chest_blink_led(
            color::f32::BLUE,
            config.chest_blink_interval,
            Priority::Medium,
        ),
        PS::Standby => nao_manager.set_chest_led(color::f32::GRAY, Priority::Critical),
        PS::Initial => nao_manager.set_chest_led(color::f32::GRAY, Priority::Critical),
        PS::Ready => nao_manager.set_chest_led(color::f32::BLUE, Priority::Critical),
        PS::Set => nao_manager.set_chest_led(color::f32::YELLOW, Priority::Critical),
        PS::Playing { .. } => nao_manager.set_chest_led(color::f32::GREEN, Priority::Critical),
        PS::Penalized => nao_manager.set_chest_led(color::f32::RED, Priority::Critical),
        PS::Finished => nao_manager.set_chest_led(color::f32::GRAY, Priority::Critical),
        PS::Calibration => nao_manager.set_chest_led(color::f32::PURPLE, Priority::Critical),
    };

    *primary_state = next_state;

    Ok(())
}

pub fn next_primary_state(
    primary_state: &PrimaryState,
    game_controller_message: &Option<GameControllerMessage>,
    chest_button: &ChestButton,
    head_buttons: &HeadButtons,
    player_config: &PlayerConfig,
    whistle_state: &WhistleState,
) -> PrimaryState {
    use PrimaryState as PS;

    let mut primary_state = match primary_state {
        PS::Unstiff if chest_button.state.is_tapped() => PS::Initial,
        PS::Initial if chest_button.state.is_tapped() => PS::Playing {
            whistle_in_set: false,
        },
        PS::Playing { .. } if chest_button.state.is_tapped() => PS::Penalized,
        PS::Penalized if chest_button.state.is_tapped() => PS::Playing {
            whistle_in_set: false,
        },

        _ => *primary_state,
    };

    // We are only able to leave the `Unstiff` state if the chest button is pressed.
    if primary_state == PS::Unstiff {
        return primary_state;
    }

    let heard_whistle = matches!(
        primary_state,
        PS::Playing {
            whistle_in_set: true
        }
    ) || whistle_state.detected;

    primary_state = match game_controller_message {
        Some(message) => match message.state {
            GameState::Initial => PS::Initial,
            GameState::Ready => PS::Ready,

            GameState::Set if heard_whistle => PS::Playing {
                whistle_in_set: true,
            },
            GameState::Set => PS::Set,
            GameState::Playing => PS::Playing {
                whistle_in_set: false,
            },
            GameState::Finished => PS::Finished,
            GameState::Standby => PS::Standby,
        },
        None => primary_state,
    };

    if is_penalized_by_game_controller(
        game_controller_message.as_ref(),
        player_config.team_number,
        player_config.player_number,
    ) {
        primary_state = PS::Penalized;
    }

    if head_buttons.all_pressed() {
        primary_state = PS::Unstiff;
    }

    primary_state
}
