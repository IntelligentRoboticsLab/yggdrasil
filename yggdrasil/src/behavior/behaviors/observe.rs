use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::time::Instant;

use crate::{
    behavior::{
        engine::{Behavior, BehaviorState},
        BehaviorConfig,
    },
    motion::{
        step_planner::StepPlanner,
        walk::engine::{Step, WalkingEngine},
    },
    nao::{NaoManager, Priority},
};
use nidhogg::types::{FillExt, HeadJoints};

const ROTATION_STIFFNESS: f32 = 0.3;

/// Config struct containing parameters for the initial behavior.
#[serde_as]
#[derive(Resource, Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ObserveBehaviorConfig {
    // Controls how fast the robot moves its head back and forth while looking around
    pub head_rotation_speed: f32,
    // Controls how far to the left and right the robot looks while looking around, in radians.
    // If this value is one, the robot will look one radian to the left and one radian to the
    // right.
    pub head_pitch_max: f32,
    // Controls how far to the bottom the robot looks while looking around, in radians
    pub head_yaw_max: f32,
}

/// This behavior makes the robot look around with a sinusoidal head movement with an optional step.
/// With this behavior, the robot can observe its surroundings while standing still or turning.
#[derive(Resource)]
pub struct Observe {
    pub starting_time: Instant,
    pub step: Option<Step>,
}

impl Default for Observe {
    fn default() -> Self {
        Observe {
            starting_time: Instant::now(),
            step: None,
        }
    }
}

impl Observe {
    #[must_use]
    pub fn with_turning(turn: f32) -> Self {
        Observe {
            starting_time: Instant::now(),
            step: Some(Step {
                turn,
                ..Default::default()
            }),
        }
    }
}

impl Behavior for Observe {
    const STATE: BehaviorState = BehaviorState::Observe;
}

pub struct ObserveBehaviorPlugin;
impl Plugin for ObserveBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            observe
                .run_if(in_state(BehaviorState::Observe))
                .run_if(resource_exists::<Observe>),
        );
    }
}

pub fn observe(
    mut nao_manager: ResMut<NaoManager>,
    behavior_config: Res<BehaviorConfig>,
    observe: Res<Observe>,
    mut step_planner: ResMut<StepPlanner>,
    mut walking_engine: ResMut<WalkingEngine>,
) {
    let observe_config = &behavior_config.observe;
    look_around(
        &mut nao_manager,
        observe.starting_time,
        observe_config.head_rotation_speed,
        observe_config.head_yaw_max,
        observe_config.head_pitch_max,
    );

    if let Some(step) = observe.step {
        step_planner.clear_target();
        walking_engine.request_walk(step);
    } else {
        walking_engine.request_stand();
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
