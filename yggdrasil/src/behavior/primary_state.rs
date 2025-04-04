use crate::{
    core::audio::whistle_detection::Whistle,
    game_controller::{penalty::PenaltyState, GameControllerMessageEvent},
    nao::{NaoManager, Priority},
    sensor::button::{ChestButton, HeadButtons},
    vision::referee::{
        communication::ReceivedRefereePose, recognize::RefereePoseRecognized, RefereePose,
    },
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

        app.add_systems(
            Update,
            (update_gamecontroller_message, update_primary_state).chain(),
        );
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
    Ready { referee_in_standby: bool },
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

fn update_gamecontroller_message(
    mut commands: Commands,
    mut events: EventReader<GameControllerMessageEvent>,
) {
    for event in events.read() {
        commands.insert_resource(**event);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn update_primary_state(
    mut primary_state: ResMut<PrimaryState>,
    game_controller_message: Option<Res<GameControllerMessage>>,
    mut nao_manager: ResMut<NaoManager>,
    (head_buttons, chest_button): (Res<HeadButtons>, Res<ChestButton>),
    config: Res<PrimaryStateConfig>,
    whistle: Res<Whistle>,
    penalty_state: Res<PenaltyState>,
    mut recognized_pose: EventReader<RefereePoseRecognized>,
    mut received_pose: EventReader<ReceivedRefereePose>,
) {
    use PrimaryState as PS;

    let next_state = next_primary_state(
        primary_state.as_mut(),
        game_controller_message.as_deref(),
        penalty_state.as_ref(),
        &chest_button,
        &head_buttons,
        &whistle,
        recognized_pose
            .read()
            .any(|event| event.pose == RefereePose::Ready)
            || received_pose
                .read()
                .any(|event| event.pose == RefereePose::Ready),
    );

    match next_state {
        PS::Sitting => nao_manager.set_chest_blink_led(
            color::f32::BLUE,
            config.chest_blink_interval,
            Priority::Medium,
        ),
        PS::Standby => nao_manager.set_chest_led(color::f32::CYAN, Priority::Critical),
        PS::Initial => nao_manager.set_chest_led(color::f32::GRAY, Priority::Critical),
        PS::Ready { .. } => nao_manager.set_chest_led(color::f32::BLUE, Priority::Critical),
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
    penalty_state: &PenaltyState,
    chest_button: &ChestButton,
    head_buttons: &HeadButtons,
    whistle: &Whistle,
    recognized_ready_pose: bool,
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

    let recognized_ready_pose = matches!(
        primary_state,
        PS::Ready {
            referee_in_standby: true
        }
    ) || recognized_ready_pose;

    primary_state = match game_controller_message {
        Some(message) => match message.state {
            GameState::Initial => PS::Initial,
            GameState::Standby if recognized_ready_pose => PS::Ready {
                referee_in_standby: true,
            },
            GameState::Ready => PS::Ready {
                referee_in_standby: false,
            },
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

    if penalty_state.is_penalized() {
        primary_state = PS::Penalized;
    }

    if head_buttons.all_pressed() {
        primary_state = PS::Sitting;
    }

    primary_state
}
