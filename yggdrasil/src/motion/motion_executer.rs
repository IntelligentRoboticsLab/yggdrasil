use crate::motion::{motion_manager::MotionManager, motion_types::Motion};
use miette::Result;
use nidhogg::{types::JointArray, NaoState};
use std::time::{Duration, SystemTime};
use tyr::prelude::*;

const STARTING_POSITION_ERROR_MARGIN: f32 = 0.05;
const LERP_TO_STARTING_POSITION_DURATION_SECS: f32 = 5.0;

/// Checks if the current position has reached the target position with a certain
/// margin of error.
///
/// # Arguments
///
/// * `current_positions` - Positions of which you want to check if they have reached a certain
///                         position.
/// * `target_positions` - Positions of which you want to check if they ahve been reached.
/// * `error_margin` - Range within which a target position has been reached.
fn reached_position(
    current_positions: &JointArray<f32>,
    target_positions: &JointArray<f32>,
    error_margin: f32,
) -> bool {
    let curr_iter = current_positions.clone().into_iter();
    let target_iter = target_positions.clone().into_iter();

    curr_iter
        .zip(target_iter)
        .map(|(curr, target)| target - error_margin <= curr && curr <= target + error_margin)
        .collect::<Vec<bool>>()
        .contains(&false)
}

/// Performs linear interpolation between two `JointArray<f32>`.
///
/// # Arguments
///
/// * `current_positions` - Starting position.
/// * `target_positions` - Final position.
/// * `scalar` - Scalar from 0-1 that indicates what weight to assign to each position.
fn lerp(
    current_positions: &JointArray<f32>,
    target_positions: &JointArray<f32>,
    scalar: f32,
) -> JointArray<f32> {
    let curr_iter = current_positions.clone().into_iter();
    let target_iter = target_positions.clone().into_iter();

    curr_iter
        .zip(target_iter)
        .map(|(curr, target)| curr * scalar + target * (1.0 - scalar))
        .collect()
}

/// TODO: docs + implementation
/// Retrieves the current position by using linear interpolation between the
/// two nearest positions based on the starting time and current time.
///
/// # Arguments
///
/// * `motion` - Current `Motion`.
fn get_positions(motion: &Motion, duration: &Duration) -> Option<JointArray<f32>> {
    motion
        .get_surrounding_frames(duration)
        .map(|(frame_a, frame_b)| {
            lerp(
                &frame_a.target_positions,
                &frame_b.target_positions,
                duration.as_secs_f32() / LERP_TO_STARTING_POSITION_DURATION_SECS,
            )
        })
}

/// Executes the current motion.
///
/// # Arguments
///
/// * `nao_state` - State of the robot.
/// * `motion_manager` - Keeps track of state needed for playing motions.
#[system]
pub fn motion_executer(nao_state: &mut NaoState, motion_manager: &mut MotionManager) -> Result<()> {
    if let Some(motion) = motion_manager.current_motion.clone() {
        if !motion_manager.started_executing_motion {
            if motion_manager.motion_starting_time.is_none() {
                motion_manager.motion_starting_time = Some(SystemTime::now());
            }

            if !reached_position(
                &nao_state.position,
                &motion.initial_positions,
                STARTING_POSITION_ERROR_MARGIN,
            ) {
                // Starting position has not yet been reached, so lerp to start position, untill
                // position has been reached.
                let elapsed_time_since_start_of_motion: f32 = motion_manager
                    .motion_starting_time
                    .unwrap()
                    .elapsed()
                    .unwrap()
                    .as_secs_f32();

                nao_state.position = lerp(
                    &nao_state.position,
                    &motion.initial_positions,
                    elapsed_time_since_start_of_motion / LERP_TO_STARTING_POSITION_DURATION_SECS,
                );

                return Ok(());
            } else {
                motion_manager.motion_execution_starting_time = Some(SystemTime::now());
            }

            match get_positions(
                &motion,
                &motion_manager
                    .motion_execution_starting_time
                    .unwrap()
                    .elapsed()
                    .unwrap(),
            ) {
                Some(position) => {
                    nao_state.position = position;
                    // TODO: Add this to the motion files.
                    nao_state.stiffness = JointArray::<f32>::default();
                }
                None => {
                    //Current motion is finished.
                    motion_manager.current_motion = None;
                    motion_manager.motion_starting_time = None;
                    motion_manager.motion_execution_starting_time = None;
                    motion_manager.started_executing_motion = false;
                }
            }
        }
    };

    Ok(())
}
