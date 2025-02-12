use bevy::prelude::*;

use crate::motion::keyframe::KeyframeExecutor;
use crate::sensor::fsr::Contacts;
use nidhogg::types::{color, FillExt, LegJoints, RightEye};
use nidhogg::NaoState;

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState},
    motion::walk::engine::WalkingEngine,
    nao::{NaoManager, Priority},
};

// The robot shouldn't do anything while in unstiff state.
const UNSTIFF_PRIORITY: Priority = Priority::Critical;

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
    locked_leg_position: Option<LegJoints<f32>>,
}

impl Behavior for Sitting {
    const STATE: BehaviorState = BehaviorState::Sitting;
}

pub fn sitting(
    mut sitting: ResMut<Sitting>,
    mut walking_engine: ResMut<WalkingEngine>,
    mut nao_manager: ResMut<NaoManager>,
    nao_state: Res<NaoState>,
    contacts: Res<Contacts>,
    keyframe_executor: Res<KeyframeExecutor>,
) {
    // Makes right eye blue.
    nao_manager.set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::default());

    if !walking_engine.is_sitting() {
        walking_engine.request_sit();
        nao_manager.unstiff_arms(UNSTIFF_PRIORITY);
        return;
    }

    // When ground contact is lost, set the current position in air.
    if !contacts.ground && !keyframe_executor.is_motion_active() {
        if sitting.locked_leg_position.is_none() {
            sitting.locked_leg_position = Some(capture_leg_position(&nao_state));
        }
        if let Some(leg_positions) = sitting.locked_leg_position.as_ref() {
            nao_manager.stiff_sit(leg_positions.clone(), Priority::High);
        }

    // Resets locked position and makes robot floppy except for hip joints in sitting position.
    } else {
        sitting.locked_leg_position = None;
        nao_manager.unstiff_sit(UNSTIFF_PRIORITY);
        nao_manager.unstiff_arms(UNSTIFF_PRIORITY);
    }
}

fn capture_leg_position(nao_state: &NaoState) -> LegJoints<f32> {
    let position = nao_state.position.clone();

    LegJoints::builder()
        .left_leg(position.left_leg_joints())
        .right_leg(position.right_leg_joints())
        .build()
}
