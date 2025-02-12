use bevy::prelude::*;

use nidhogg::types::{color, FillExt, LegJoints, RightEye};
use nidhogg::NaoState;

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState},
    motion::walkv4::step_manager::StepManager,
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
#[derive(Resource, Default)]
pub struct Sitting {
    /// Stores the initial leg position when ground contact is lost.
    _locked_leg_position: Option<LegJoints<f32>>,
}

impl Behavior for Sitting {
    const STATE: BehaviorState = BehaviorState::Sitting;
}

pub fn sitting(mut step_manager: ResMut<StepManager>, mut nao_manager: ResMut<NaoManager>) {
    // Makes right eye blue.
    nao_manager.set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::default());

    step_manager.request_sit();
}

fn _capture_leg_position(nao_state: &NaoState) -> LegJoints<f32> {
    let position = nao_state.position.clone();

    LegJoints::builder()
        .left_leg(position.left_leg_joints())
        .right_leg(position.right_leg_joints())
        .build()
}
