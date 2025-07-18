use crate::{
    behavior::primary_state,
    core::audio::whistle_detection::{self, Whistle},
    game_controller::{GameControllerMessageEvent, penalty::PenaltyState},
    kinematics::Kinematics,
    motion::walking_engine::config::WalkingEngineConfig,
    nao::{NaoManager, Priority},
    sensor::button::{ChestButton, HeadButtons},
    vision::referee::{
        RefereePose, communication::ReceivedRefereePose, recognize::RefereePoseRecognized,
    },
};
use bevy::prelude::*;

use serde::{Deserialize, Serialize};
use serde_with::{DurationMilliSeconds, serde_as};
use std::time::Duration;

use bifrost::communication::{GameControllerMessage, GameState};
use nidhogg::types::color;

use std::time::Instant;

const GOAL_DELAY: u64 = 15;

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

#[derive(Default)]
pub struct WhistleTimer {
    whistle_timer: Option<Instant>,
}

impl Plugin for PrimaryStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, init_primary_state);
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
    Ready {
        referee_in_standby: bool,
        whistle_in_playing: bool,
    },
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

fn init_primary_state(
    mut commands: Commands,
    config: Res<WalkingEngineConfig>,
    kinematics: Res<Kinematics>,
) {
    let hip_height = kinematics.left_hip_height();

    if hip_height >= config.hip_height.max_sitting_hip_height {
        commands.insert_resource(PrimaryState::Initial);
    } else {
        commands.insert_resource(PrimaryState::Sitting);
    }
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
    mut whistle_timer: Local<WhistleTimer>,
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
        &mut whistle_timer,
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
    whistle_timer: &mut WhistleTimer,
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

    // makes sure "set" is not skipped
    if let Some(GameControllerMessage {
        state: GameState::Set,
        ..
    }) = game_controller_message
    {
        if matches!(
            primary_state,
            PS::Ready {
                whistle_in_playing: true,
                ..
            }
        ) {
            primary_state = PS::Ready {
                referee_in_standby: false,
                whistle_in_playing: false,
            };
        }
    }

    let heard_whistle = matches!(
        primary_state,
        PS::Playing {
            whistle_in_set: true
        }
    ) || matches!(
        primary_state,
        PS::Ready {
            whistle_in_playing: true,
            ..
        }
    ) || whistle.detected();

    let recognized_ready_pose = matches!(
        primary_state,
        PS::Ready {
            referee_in_standby: true,
            ..
        }
    ) || recognized_ready_pose;

    let previous_primary_state = primary_state;
    primary_state = match game_controller_message {
        Some(message) => match message.state {
            GameState::Initial => PS::Initial,
            GameState::Standby if recognized_ready_pose => PS::Ready {
                referee_in_standby: true,
                whistle_in_playing: false,
            },
            GameState::Ready => PS::Ready {
                referee_in_standby: false,
                whistle_in_playing: false,
            },
            GameState::Set if heard_whistle => PS::Playing {
                whistle_in_set: true,
            },
            GameState::Set => PS::Set,
            GameState::Playing if heard_whistle => PS::Ready {
                referee_in_standby: false,
                whistle_in_playing: true,
            },
            GameState::Playing => PS::Playing {
                whistle_in_set: false,
            },
            GameState::Finished => PS::Finished,
            GameState::Standby => PS::Standby,
        },
        None => primary_state,
    };

    // set a timer after switching to the ready state due to a whistle
    if previous_primary_state != primary_state
        && matches!(
            primary_state,
            PS::Ready {
                whistle_in_playing: true,
                ..
            }
        )
    {
        whistle_timer.whistle_timer = Some(Instant::now());
    }

    // check if a timer was set
    if let Some(start_time) = whistle_timer.whistle_timer {
        // if it lasted over GOAL_DELAY it should be reset
        // either it was a false positive, or it was okay and should also be reset
        if start_time.elapsed().as_secs() > GOAL_DELAY {
            whistle_timer.whistle_timer = None;
            // if it still has whistle_in_playing:true after the GOAL_DELAY, change to false
            if matches! (primary_state, PS::Ready {whistle_in_playing:true, .. }) {
                primary_state = PS::Ready {
                    referee_in_standby: false,
                    whistle_in_playing: false,
                };
            }
        }
    }

    if previous_primary_state != primary_state {
        println!("primary state is set to: {:?}", primary_state);
    }

    if penalty_state.is_penalized() {
        primary_state = PS::Penalized;
    }

    if head_buttons.all_pressed() {
        primary_state = PS::Sitting;
    }

    primary_state
}
