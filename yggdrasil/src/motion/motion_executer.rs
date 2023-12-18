use crate::motion::motion_manager::{ActiveMotion, MotionManager};
use crate::motion::motion_util::{lerp, MotionUtilExt};
use miette::Result;
use nidhogg::{
    types::{FillExt, JointArray},
    NaoControlMessage, NaoState,
};
use std::time::SystemTime;
use tyr::prelude::*;

const STARTING_POSITION_ERROR_MARGIN: f32 = 0.40;
const LERP_TO_STARTING_POSITION_DURATION_SECS: f32 = 0.5;
const STIFFNESS: f32 = 0.8;

/// Checks if the current position has reached the target position with a certain
/// margin of error.
///
/// # Arguments
///
/// * `current_position` - Position of which you want to check if it has reached a certain
///                        position.
/// * `target_position` - Position of which you want to check if it has been reached.
/// * `error_margin` - Range within which a target position has been reached.
pub fn reached_position(
    current_position: &JointArray<f32>,
    target_position: &JointArray<f32>,
    error_margin: f32,
) -> bool {
    let mut t = current_position
        .clone()
        .zip(target_position.clone())
        .map(|(curr, target)| target - error_margin <= curr && curr <= target + error_margin);

    println!(
        "CHECK1 {:?} {:?} {:?}",
        current_position.right_shoulder_pitch,
        target_position.right_shoulder_pitch,
        t.right_shoulder_pitch
    );

    println!(
        "CHECK {:?} {:?} {:?}",
        current_position.left_shoulder_pitch,
        target_position.left_shoulder_pitch,
        t.left_shoulder_pitch
    );

    // Ignore hands.
    t.left_hand = true;
    t.right_hand = true;
    t.all(|elem| elem == true)
}

/// Executes the current motion.
///
/// # Arguments
///
/// * `nao_state` - State of the robot.
/// * `motion_manager` - Keeps track of state needed for playing motions.
/// * `nao_control_message` - Used to set the new joint positions.
#[system]
pub fn motion_executer(
    nao_state: &mut NaoState,
    motion_manager: &mut MotionManager,
    nao_control_message: &mut NaoControlMessage,
) -> Result<()> {
    if motion_manager.active_motion.is_none() {
        return Ok(());
    }

    let ActiveMotion {
        motion,
        starting_time,
    } = motion_manager.get_active_motion().unwrap();

    if motion_manager.motion_execution_starting_time.is_none() {
        if !reached_position(
            &nao_state.position,
            &motion.initial_position,
            STARTING_POSITION_ERROR_MARGIN,
        ) {
            println!("Not reached starting position");
            // Starting position has not yet been reached, so lerp to start
            // position, until position has been reached.
            let elapsed_time_since_start_of_motion: f32 =
                starting_time.elapsed().unwrap().as_secs_f32();

            nao_control_message.position = lerp(
                &nao_state.position,
                &motion.initial_position,
                elapsed_time_since_start_of_motion / LERP_TO_STARTING_POSITION_DURATION_SECS,
            );
            nao_control_message.stiffness = JointArray::<f32>::fill(STIFFNESS);

            return Ok(());
        } else {
            println!("Reached starting position");
            motion_manager.motion_execution_starting_time = Some(SystemTime::now());
        }
    }

    let motion_duration = motion_manager
        .motion_execution_starting_time
        .unwrap()
        .elapsed()
        .unwrap();

    match motion.get_position(motion_duration) {
        Some(position) => {
            nao_control_message.position = position;
            // TODO: Add stiffness to the motion files.
            nao_control_message.stiffness = JointArray::<f32>::fill(0.5);
        }
        None => {
            //Current motion is finished.
            motion_manager.active_motion = None;
            motion_manager.motion_execution_starting_time = None;
        }
    }

    Ok(())
}
