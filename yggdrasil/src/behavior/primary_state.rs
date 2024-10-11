use crate::{
    core::{audio::whistle_detection::Whistle, config::showtime::PlayerConfig},
    nao::{NaoManager, Priority},
    sensor::button::{ChestButton, HeadButtons},
};
use bevy::prelude::*;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};
use std::time::Duration;

use bifrost::communication::{GameControllerMessage, GameState};
use nidhogg::types::color;

#[serde_as]
#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct PrimaryStateConfig {
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub chest_blink_interval: Duration,
}

/// Plugin providing information about the primary state of the robot. These
/// states include: "Unstiff", "Initial", "Ready", "Set", "Playing",
/// "Penalized", "Finished" and "Calibration".
///
/// This module provides the following resources to the application:
/// - [`PrimaryState`]
pub struct PrimaryStatePlugin;

impl Plugin for PrimaryStatePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PrimaryState::Sitting);

        app.add_systems(Update, update_primary_state);
    }
}

#[derive(Resource, Debug, Clone, PartialEq, Copy, Default, Reflect)]
pub enum PrimaryState {
    /// State in which all joints but the hips are unstiffened
    /// and the robot does not move, sitting down.
    #[default]
    Sitting,
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
    /// State the indicates the robot is performing automatic calibration
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
            .is_some_and(|team| team.is_penalized(player_number))
    })
}

pub fn update_primary_state(
    mut primary_state: ResMut<PrimaryState>,
    game_controller_message: Option<Res<GameControllerMessage>>,
    mut nao_manager: ResMut<NaoManager>,
    (head_buttons, chest_button): (Res<HeadButtons>, Res<ChestButton>),
    config: Res<PrimaryStateConfig>,
    player_config: Res<PlayerConfig>,
    whistle: Res<Whistle>,
) {
    use PrimaryState as PS;
    let next_state = next_primary_state(
        primary_state.as_mut(),
        game_controller_message.as_deref(),
        &chest_button,
        &head_buttons,
        &player_config,
        &whistle,
    );

    match next_state {
        PS::Sitting => nao_manager.set_chest_blink_led(
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
}

#[must_use]
pub fn next_primary_state(
    primary_state: &PrimaryState,
    game_controller_message: Option<&GameControllerMessage>,
    chest_button: &ChestButton,
    head_buttons: &HeadButtons,
    player_config: &PlayerConfig,
    whistle: &Whistle,
) -> PrimaryState {
    use PrimaryState as PS;

    let mut primary_state = match primary_state {
        PS::Sitting if chest_button.state.is_tapped() => PS::Initial,
        PS::Initial if chest_button.state.is_tapped() => PS::Playing {
            whistle_in_set: false,
        },
        PS::Playing { .. } if chest_button.state.is_tapped() => PS::Penalized,
        PS::Penalized if chest_button.state.is_tapped() => PS::Playing {
            whistle_in_set: false,
        },

        _ => *primary_state,
    };

    // We are only able to leave the `Sitting` state if the chest button is pressed.
    if primary_state == PS::Sitting {
        return primary_state;
    }

    let heard_whistle = matches!(
        primary_state,
        PS::Playing {
            whistle_in_set: true
        }
    ) || whistle.detected();

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
        game_controller_message,
        player_config.team_number,
        player_config.player_number,
    ) {
        primary_state = PS::Penalized;
    }

    if head_buttons.all_pressed() {
        primary_state = PS::Sitting;
    }

    primary_state
}
