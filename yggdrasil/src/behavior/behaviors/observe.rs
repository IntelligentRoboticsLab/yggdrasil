use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use std::time::Instant;

use crate::{
    behavior::engine::{Behavior, BehaviorState, in_behavior},
    motion::walking_engine::{StandingHeight, step::Step, step_context::StepContext},
    nao::{HeadMotionManager, NaoManager},
};

/// Config struct containing parameters for the initial behavior.
#[serde_as]
#[derive(Resource, Serialize, Deserialize, Debug, Clone, Default)]
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
    // The look at head stiffness
    pub look_at_head_stiffness: f32,
    // The look around head stiffness
    pub look_around_head_stiffness: f32,
}

#[derive(Resource, Deref)]
struct ObserveStartingTime(Instant);

/// This behavior makes the robot look around with a sinusoidal head movement with an optional step.
/// With this behavior, the robot can observe its surroundings while standing still or turning.
#[derive(Resource, Default)]
pub struct Observe {
    pub step: Option<Step>,
}

impl Observe {
    #[must_use]
    pub fn with_turning(turn: f32) -> Self {
        Observe {
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
        app.add_systems(Update, observe.run_if(in_behavior::<Observe>))
            .add_systems(OnEnter(BehaviorState::Observe), reset_observe_starting_time)
            .insert_resource(ObserveStartingTime(Instant::now()));
    }
}

fn reset_observe_starting_time(mut observe_starting_time: ResMut<ObserveStartingTime>) {
    observe_starting_time.0 = Instant::now();
}

fn observe(
    mut nao_manager: ResMut<NaoManager>,
    observe: Res<Observe>,
    observe_starting_time: Res<ObserveStartingTime>,
    mut step_context: ResMut<StepContext>,
    head_motion_manager: Res<HeadMotionManager>,
) {
    head_motion_manager.look_around(&mut nao_manager, **observe_starting_time);

    if let Some(step) = observe.step {
        step_context.request_walk(step);
    } else {
        step_context.request_stand_with_height(StandingHeight::MAX);
    }
}
