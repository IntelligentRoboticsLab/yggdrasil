use bevy::prelude::*;

use nidhogg::types::{color, FillExt, RightEye};
use std::time::{Duration, Instant};

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState},
    motion::walking_engine::{step_context::StepContext, Gait},
    nao::{NaoManager, Priority},
};

const MIN_SITTING_DURATION_BEFORE_UNSTIFF: Duration = Duration::from_secs(5);

pub struct SittingBehaviorPlugin;

impl Plugin for SittingBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sitting.run_if(in_behavior::<Sitting>))
            .add_systems(OnEnter(Gait::Sitting), reset_sitting_instant)
            .insert_resource(SittingInstant(Instant::now()));
    }
}

/// This is often the starting behavior of the robot.
/// In this state the robot sits down, after which it unstiffens its legs, arms and head.
#[derive(Resource)]
pub struct Sitting;

impl Behavior for Sitting {
    const STATE: BehaviorState = BehaviorState::Sitting;
}

#[derive(Resource)]
pub struct SittingInstant(Instant);

fn reset_sitting_instant(mut time_since_sitting: ResMut<SittingInstant>) {
    time_since_sitting.0 = Instant::now();
}

pub fn sitting(
    mut step_context: ResMut<StepContext>,
    mut nao_manager: ResMut<NaoManager>,
    gait: Res<State<Gait>>,
    time_since_sitting: Res<SittingInstant>,
) {
    nao_manager.set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::default());

    match &gait.get() {
        Gait::Sitting => {
            if time_since_sitting.0.elapsed() > MIN_SITTING_DURATION_BEFORE_UNSTIFF {
                nao_manager.unstiff_sit(Priority::High);
            }
        }
        _ => {
            step_context.request_sit();
        }
    }
}
