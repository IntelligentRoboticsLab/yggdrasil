use super::InterpolationType;
use super::{
    get_min_duration, interpolate_jointarrays, types::Movement, ActiveMotion, KeyframeExecutor,
};
use crate::nao::NaoManager;
use crate::nao::Priority;
use bevy::prelude::*;
use miette::{miette, Result};
use nidhogg::types::{ArmJoints, HeadJoints, LegJoints};
use nidhogg::{
    types::{FillExt, JointArray},
    NaoState,
};
use std::time::{Duration, Instant};

// maximum speed the robot is allowed to move to the starting position at
const MAX_SPEED: f32 = 1.0;

/// Executes the current motion.
///
/// # Arguments
/// * `nao_state` - State of the robot.
/// * `keyframe_executor` - Keeps track of state needed for playing motions.
/// * `nao_manager` - Used to set the new joint positions.
#[allow(clippy::too_many_lines)]
pub fn keyframe_executor(
    nao_state: ResMut<NaoState>,
    mut keyframe_executor: ResMut<KeyframeExecutor>,
    mut nao_manager: ResMut<NaoManager>,
) {
    if keyframe_executor.active_motion.is_none() {
        return;
    }

    // keeping track of the moment that the current motion has started
    if keyframe_executor.motion_execution_starting_time.is_none() {
        keyframe_executor.motion_execution_starting_time = Some(Instant::now());
    }

    // setting up some variables that are used frequently
    let ActiveMotion {
        motion,
        cur_sub_motion: (sub_motion_name, _),
        movement_start,
        ..
    } = keyframe_executor
        .active_motion
        .clone()
        .ok_or_else(|| {
            keyframe_executor.stop_motion();
            miette!("KeyframeExecutor.ActiveMotion could not be cloned, likely contained None")
        })
        .expect("failed to clone active motion");

    let submotion_stiffness: f32 = motion.submotions[&sub_motion_name].joint_stifness;

    // at the start of a new submotion, we need to lerp to the starting position
    if keyframe_executor
        .submotion_execution_starting_time
        .is_none()
    {
        let Movement {
            target_position,
            duration,
        } = &motion.initial_movement(&sub_motion_name);

        // before beginning the first movement, we have to prepare the movement to avoid damage
        if keyframe_executor.source_position.is_none() {
            // record the last position before motion initialization, or before transition
            keyframe_executor.source_position = Some(nao_state.position.clone());
            prepare_initial_movement(
                &mut keyframe_executor,
                target_position,
                duration,
                &sub_motion_name,
            )
            .expect("failed to prepare initial movement");
        }

        // getting the next position for the robot
        if let Some(next_position) = move_to_starting_position(
            &keyframe_executor,
            target_position,
            duration,
            &movement_start.elapsed(),
            &motion.settings.interpolation_type,
        ) {
            nao_manager.set_all(
                next_position,
                HeadJoints::<f32>::fill(submotion_stiffness),
                ArmJoints::<f32>::fill(submotion_stiffness),
                LegJoints::<f32>::fill(submotion_stiffness),
                Priority::High,
            );
        } else {
            // if the starting position has been reached,
            // we update the active motion for executing the submotion
            update_active_motion(&mut keyframe_executor);
        }
    }

    // set next joint positions
    if let Some(position) = motion.get_position(
        &sub_motion_name,
        keyframe_executor.active_motion.as_mut().unwrap(),
    ) {
        nao_manager.set_all(
            position,
            HeadJoints::<f32>::fill(submotion_stiffness),
            ArmJoints::<f32>::fill(submotion_stiffness),
            LegJoints::<f32>::fill(submotion_stiffness),
            Priority::High,
        );
    } else {
        transition_to_next_submotion(&mut keyframe_executor)
            .inspect_err(|_| keyframe_executor.stop_motion())
            .expect("failed to transition to next submotion");

        nao_manager.set_all(
            nao_state.position.clone(),
            HeadJoints::<f32>::fill(submotion_stiffness),
            ArmJoints::<f32>::fill(submotion_stiffness),
            LegJoints::<f32>::fill(submotion_stiffness),
            Priority::High,
        );
    }
}

/// Prepares the initial movement of a submotion.
///
///
/// # Notes
/// Currently only checks and possibly edits the movement duration to prevent dangerously
/// quick movements, but will be expanded upon.
///
/// # Arguments
/// * `keyframe_executor` - Keeps track of state needed for playing motions.
/// * `target_position` - The target position of the initial movement.
/// * `duration` - Intended duration of the initial movement.
/// * `sub_motion_name` - Current submotion to be executed.
fn prepare_initial_movement(
    keyframe_executor: &mut KeyframeExecutor,
    target_position: &JointArray<f32>,
    duration: &Duration,
    sub_motion_name: &String,
) -> Result<()> {
    // checking whether the given duration will exceed our maximum speed limit
    let min_duration = get_min_duration(
        keyframe_executor
            .source_position
            .as_ref()
            .ok_or_else(|| miette!("Getting the source position failed during initial movement"))?,
        target_position,
        MAX_SPEED,
    );
    if duration > &min_duration {
        // editing the movement duration to prevent dangerously quick movements
        keyframe_executor
            .active_motion
            .as_mut()
            .unwrap()
            .motion
            .set_initial_duration(sub_motion_name, min_duration);
    }

    Ok(())
}

/// Updates the active motion to begin executing the current submotion.
///
/// # Arguments
/// * `keyframe_executor` - Keeps track of state needed for playing motions.
fn update_active_motion(keyframe_executor: &mut KeyframeExecutor) {
    // update the time of the start of the movement
    keyframe_executor.submotion_execution_starting_time = Some(Instant::now());
    keyframe_executor
        .active_motion
        .as_mut()
        .unwrap()
        .movement_start = Instant::now();
    keyframe_executor
        .active_motion
        .as_mut()
        .unwrap()
        .cur_keyframe_index += 1;
}

/// Calculates the next position of the robot to approach the starting position.
/// If the robot has reached the starting position, it will return None.
///
/// # Notes
/// Currently the function is still quite barren, but this will be expanded upon later.
/// For example, different interpolation types will be available.
///
/// # Arguments
/// * `keyframe_executor` - Keeps track of state needed for playing motions.
/// * `target_position` - The target position of the initial movement.
/// * `duration` - Intended duration of the initial movement.
/// * `elapsed_time` - Currently elapsed time since start of movement to initial position.
fn move_to_starting_position(
    keyframe_executor: &KeyframeExecutor,
    target_position: &JointArray<f32>,
    duration: &Duration,
    elapsed_time_since_start_of_motion: &Duration,
    interpolation_type: &InterpolationType,
) -> Option<JointArray<f32>> {
    if elapsed_time_since_start_of_motion <= duration {
        return Some(interpolate_jointarrays(
            keyframe_executor.source_position.as_ref().unwrap(),
            target_position,
            elapsed_time_since_start_of_motion.as_secs_f32() / duration.as_secs_f32(),
            interpolation_type,
        ));
    }

    None
}

/// Handles the logic for transitioning to the next submotion.
/// If a submotion is present, will transition to this submotion.
/// If not, will reset the active motion and saved time values.
///
/// # Arguments
/// * `keyframe_executor` - Keeps track of state needed for playing motions.
fn transition_to_next_submotion(keyframe_executor: &mut KeyframeExecutor) -> Result<()> {
    // current submotion is finished, transition to next submotion.
    let active_motion: &mut ActiveMotion =
        keyframe_executor.active_motion.as_mut().ok_or_else(|| {
            miette!("No active motion present during transition, have you started a motion?")
        })?;

    keyframe_executor.submotion_execution_starting_time = None;
    keyframe_executor.submotion_finishing_time = None;
    keyframe_executor.source_position = None;

    if let Some(submotion_name) = active_motion.get_next_submotion() {
        // If there is a next submotion, we attempt a transition
        let next_submotion = active_motion.transition(submotion_name.clone())?;
        keyframe_executor.active_motion = next_submotion;

        Ok(())
    }
    // if no submotion is found, the motion has finished
    else {
        // we reset the KeyframeExecutor
        keyframe_executor.active_motion = None;
        keyframe_executor.motion_execution_starting_time = None;

        Ok(())
    }
}
