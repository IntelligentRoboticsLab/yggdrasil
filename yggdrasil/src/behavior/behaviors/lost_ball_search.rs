use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::time::Instant;

use crate::{
    behavior::{
        BehaviorConfig,
        engine::{Behavior, BehaviorState, in_behavior},
    },
    motion::walking_engine::{StandingHeight, step::Step, step_context::StepContext},
    nao::{NaoManager, Priority},
};
use nidhogg::types::{FillExt, HeadJoints};

const ROTATION_STIFFNESS: f32 = 0.3;

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
    mut nao_manager: ResMut<NaoManager>,
    behavior_config: Res<BehaviorConfig>,
    observe: Res<LostBallSearch>,
    observe_starting_time: Res<LostBallSearchStartingTime>,
    mut step_context: ResMut<StepContext>,
) {
    let observe_config = &behavior_config.observe;
    look_around(
        &mut nao_manager,
        **observe_starting_time,
        observe_config.head_rotation_speed,
        observe_config.head_yaw_max,
        observe_config.head_pitch_max,
    );

    if let Some(step) = observe.step {
        step_context.request_walk(step);
    } else {
        step_context.request_stand_with_height(StandingHeight::MAX);
    }
}

fn look_around(
    nao_manager: &mut NaoManager,
    starting_time: Instant,
    rotation_speed: f32,
    yaw_multiplier: f32,
    pitch_multiplier: f32,
) {
    // Used to parameterize the yaw and pitch angles, multiplying with a large
    // rotation speed will make the rotation go faster.
    let movement_progress = starting_time.elapsed().as_secs_f32() * rotation_speed;
    let yaw = (movement_progress).sin() * yaw_multiplier;
    let pitch = (movement_progress * 2.0 + std::f32::consts::FRAC_PI_2)
        .sin()
        .max(0.0)
        * pitch_multiplier;

    let position = HeadJoints { yaw, pitch };
    let stiffness = HeadJoints::fill(ROTATION_STIFFNESS);

    nao_manager.set_head(position, stiffness, Priority::default());
}
