use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::time::Instant;

use crate::{
    behavior::engine::{Behavior, BehaviorState, in_behavior},
    motion::walking_engine::{StandingHeight, step::Step, step_context::StepContext},
    nao::HeadMotionManager,
};

/// Config struct containing parameters for the initial behavior.
#[serde_as]
#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct LostBallSearchBehaviorConfig {
    // Controls how fast the robot moves its head back and forth while looking around
    pub head_rotation_speed: f32,
    // Controls how far to the left and right the robot looks while looking around, in radians.
    // If this value is one, the robot will look one radian to the left and one radian to the
    // right.
    pub head_pitch_max: f32,
    // Controls how far to the bottom the robot looks while looking around, in radians
    pub head_yaw_max: f32,
}

#[derive(Resource, Deref)]
struct LostBallSearchStartingTime(Instant);

/// This behavior makes the robot look around with a sinusoidal head movement with an optional step.
/// With this behavior, the robot can observe its surroundings while standing still or turning.
#[derive(Resource, Default)]
pub struct LostBallSearch {
    pub step: Option<Step>,
}

impl LostBallSearch {
    #[must_use]
    pub fn with_turning(turn: f32) -> Self {
        LostBallSearch {
            step: Some(Step {
                turn,
                ..Default::default()
            }),
        }
    }
}

impl Behavior for LostBallSearch {
    const STATE: BehaviorState = BehaviorState::LostBallSearch;
}

pub struct LostBallSearchBehaviorPlugin;

impl Plugin for LostBallSearchBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, observe.run_if(in_behavior::<LostBallSearch>))
            .add_systems(
                OnEnter(BehaviorState::Observe),
                reset_lost_ball_search_starting_time,
            )
            .insert_resource(LostBallSearchStartingTime(Instant::now()));
    }
}

fn reset_lost_ball_search_starting_time(
    mut lost_ball_search_starting_time: ResMut<LostBallSearchStartingTime>,
) {
    lost_ball_search_starting_time.0 = Instant::now();
}

fn observe(
    observe: Res<LostBallSearch>,
    mut step_context: ResMut<StepContext>,
    mut head_motion_manager: ResMut<HeadMotionManager>,
) {
    head_motion_manager.request_look_around();

    if let Some(step) = observe.step {
        step_context.request_walk(step);
    } else {
        step_context.request_stand_with_height(StandingHeight::MAX);
    }
}
