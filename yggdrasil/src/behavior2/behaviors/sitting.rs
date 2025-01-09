use bevy::prelude::*;

use nidhogg::types::{color, FillExt, RightEye};

use crate::{
    behavior2::engine::{Behavior, BehaviorState},
    impl_behavior,
    motion::walk::engine::WalkingEngine,
    nao::{NaoManager, Priority},
};

pub struct SittingBehaviorPlugin;

impl Plugin for SittingBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sitting.run_if(in_state(BehaviorState::Sitting)));
    }
}

/// This is often the starting behavior of the robot.
/// In this state the robot sits down, after which it unstiffens its legs, arms and head.
#[derive(Resource)]
pub struct Sitting;

impl_behavior!(Sitting, Sitting);

// The robot shouldn't do anything while in unstiff state.
const UNSTIFF_PRIORITY: Priority = Priority::Critical;

pub fn sitting(mut walking_engine: ResMut<WalkingEngine>, mut nao_manager: ResMut<NaoManager>) {
    // Makes right eye blue.
    nao_manager.set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::default());

    if walking_engine.is_sitting() {
        // Makes robot floppy except for hip joints, locked in sitting position.
        nao_manager.unstiff_sit(UNSTIFF_PRIORITY);
    } else {
        walking_engine.request_sit();
    }

    nao_manager.unstiff_arms(UNSTIFF_PRIORITY);
}
