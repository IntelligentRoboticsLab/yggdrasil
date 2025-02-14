use bevy::prelude::*;

use nidhogg::types::{color, FillExt, RightEye};

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState},
    motion::walkv4::step_manager::StepContext,
    nao::{NaoManager, Priority},
};

pub struct SittingBehaviorPlugin;

impl Plugin for SittingBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sitting.run_if(in_behavior::<Sitting>));
    }
}

/// This is often the starting behavior of the robot.
/// In this state the robot sits down, after which it unstiffens its legs, arms and head.
#[derive(Resource)]
pub struct Sitting;

impl Behavior for Sitting {
    const STATE: BehaviorState = BehaviorState::Sitting;
}

pub fn sitting(mut step_context: ResMut<StepContext>, mut nao_manager: ResMut<NaoManager>) {
    // Makes right eye blue.
    nao_manager.set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::default());
    step_context.request_sit();
}
