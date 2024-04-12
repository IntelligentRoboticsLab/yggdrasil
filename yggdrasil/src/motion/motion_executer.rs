use crate::motion::motion_manager::{ActiveMotion, MotionManager};
use crate::motion::motion_types::Movement;
use crate::motion::motion_util::{get_min_duration, lerp};
use crate::nao::manager::NaoManager;
use crate::nao::manager::Priority;
use miette::Result;
use nidhogg::{
    types::{FillExt, JointArray},
    NaoState,
};
use std::time::{Duration, Instant};
use tyr::prelude::*;

const MAX_SPEED: f32 = 1.0;

/// Executes the current motion.
///
/// # Arguments
///
/// * `nao_state` - State of the robot.
/// * `motion_manager` - Keeps track of state needed for playing motions.
/// * `nao_manager` - Used to set the new joint positions.
#[system]
pub fn motion_executer(
    nao_state: &mut NaoState,
    motion_manager: &mut MotionManager,
    nao_manager: &mut NaoManager,
) -> Result<()> {
    if motion_manager.active_motion.is_none() {
        return Ok(());
    }

    // keeping track of the moment that the current motion has started
    if motion_manager.motion_execution_starting_time.is_none() {
        motion_manager.motion_execution_starting_time = Some(Instant::now());
    }

    // setting up some variables that are used frequently
    let ActiveMotion {
        motion,
        cur_sub_motion: (sub_motion_name, _),
        cur_keyframe_index: _,
        movement_start,
        priority: _,
    } = motion_manager.active_motion.clone().unwrap();

    let submotion_stiffness: f32 = motion.submotions[&sub_motion_name].joint_stifness;

    // at the start of a new submotion, we need to lerp to the starting position
    if motion_manager.submotion_execution_starting_time.is_none() {
        let Movement {
            target_position,
            duration,
        } = &motion.initial_movement(&sub_motion_name);

        // before beginning the first movement, we have to prepare the movement to avoid damage
        if motion_manager.source_position.is_none() {
            // record the last position before motion initialization, or before transition
            motion_manager.source_position = Some(nao_state.position.clone());
            prepare_initial_movement(motion_manager, target_position, duration, &sub_motion_name);
        }

        // getting the next position for the robot
        if let Some(next_position) = move_to_starting_position(
            &motion_manager,
            &movement_start.elapsed(),
            duration,
            target_position,
        ) {
            nao_manager.set_all(
                next_position,
                JointArray::<f32>::fill(submotion_stiffness),
                Priority::High,
            );
        } else {
            // if the starting position has been reached,
            // we update the active motion for executing the submotion
            update_active_motion(motion_manager);
        }
    }

    // set next joint positions
    if let Some(position) = motion.get_position(
        &sub_motion_name,
        &mut motion_manager.active_motion.as_mut().unwrap(),
    ) {
        nao_manager.set_all(
            position,
            JointArray::<f32>::fill(submotion_stiffness),
            Priority::High,
        );
    } else {
        if !exit_waittime_elapsed(
            motion_manager,
            motion.submotions[&sub_motion_name].exit_waittime,
        ) {
            return Ok(());
        }

        transition_to_next_submotion(motion_manager, nao_state);
    }

    Ok(())
}

fn prepare_initial_movement(
    motion_manager: &mut MotionManager,
    target_position: &JointArray<f32>,
    duration: &Duration,
    sub_motion_name: &String,
) {
    // checking whether the given duration will exceed our maximum speed limit
    let min_duration = get_min_duration(
        motion_manager.source_position.as_ref().unwrap(),
        target_position,
        MAX_SPEED,
    );
    if duration > &min_duration {
        // editing the movement duration to prevent dangerously quick movements                                            TODO SIMPLIFY WITH FUNC
        motion_manager
            .active_motion
            .as_mut()
            .unwrap()
            .motion
            .set_initial_duration(sub_motion_name, min_duration);
    }
}

fn update_active_motion(motion_manager: &mut MotionManager) {
    // update the time of the start of the movement
    motion_manager.submotion_execution_starting_time = Some(Instant::now());
    motion_manager
        .active_motion
        .as_mut()
        .unwrap()
        .movement_start = Instant::now();
    motion_manager
        .active_motion
        .as_mut()
        .unwrap()
        .cur_keyframe_index += 1;
}

fn move_to_starting_position(
    motion_manager: &MotionManager,
    elapsed_time_since_start_of_motion: &Duration,
    duration: &Duration,
    target_position: &JointArray<f32>,
) -> Option<JointArray<f32>> {
    if elapsed_time_since_start_of_motion <= duration {
        return Some(lerp(
            motion_manager.source_position.as_ref().unwrap(),
            target_position,
            elapsed_time_since_start_of_motion.as_secs_f32() / duration.as_secs_f32(),
        ));
    }

    None
}

fn exit_waittime_elapsed(motion_manager: &mut MotionManager, exit_waittime: f32) -> bool {
    if exit_waittime <= 0.05 {
        return true;
    }

    // firstly, we record the current timestamp and check whether the motion needs to wait
    if let Some(finishing_time) = motion_manager.submotion_finishing_time {
        // checking whether the required waittime has elapsed
        if finishing_time.elapsed().as_secs_f32() < exit_waittime {
            return false;
        }

        return true;
    } else {
        motion_manager.submotion_finishing_time = Some(Instant::now());
        return false;
    }
}

fn transition_to_next_submotion(motion_manager: &mut MotionManager, nao_state: &mut NaoState) {
    // current submotion is finished, transition to next submotion.
    let active_motion: &mut ActiveMotion = motion_manager.active_motion.as_mut().unwrap();

    motion_manager.submotion_execution_starting_time = None;
    motion_manager.submotion_finishing_time = None;
    motion_manager.source_position = None;

    if let Some(submotion_name) = active_motion.get_next_submotion() {
        // If there is a next submotion, we attempt a transition
        let next_submotion = active_motion.transition(nao_state, submotion_name.clone());
        motion_manager.active_motion = next_submotion;

        // if the motion was aborted or an error occured with transitioning, we reset the execution time
        if motion_manager.active_motion.is_none() {
            motion_manager.motion_execution_starting_time = None;
        }
    } else {
        // if no submotion is found, the motion has finished
        motion_manager.active_motion = None;
        motion_manager.motion_execution_starting_time = None;
    }
}
