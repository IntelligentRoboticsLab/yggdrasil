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
use std::time::Instant;
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
        cur_sub_motion,
        cur_keyframe_index: _,
        movement_start,
    } = motion_manager.get_active_motion().unwrap();

    let sub_motion_name: &String = &cur_sub_motion.0;
    let submotion_stiffness: f32 = motion.submotions[sub_motion_name].joint_stifness;

    // at the start of a new submotion, we need to lerp to the starting position
    if motion_manager.submotion_execution_starting_time.is_none() {
        let Movement {
            target_position,
            duration,
        } = &motion.initial_movement(sub_motion_name);

        // record the last position before motion initialization, or before transition
        if motion_manager.source_position.is_none() {
            motion_manager.source_position = Some(nao_state.position.clone());

            // checking whether the given duration will exceed our maximum speed limit
            let min_duration = get_min_duration(
                motion_manager.source_position.as_ref().unwrap(),
                target_position,
                MAX_SPEED,
            );
            if duration > &min_duration {
                println!("MOTION TOO QUICK!");
                motion_manager
                    .active_motion
                    .as_mut()
                    .unwrap()
                    .motion
                    .set_initial_duration(sub_motion_name, min_duration);
            }
        }

        // Starting position has not yet been reached, so lerp to start
        // position, until position has been reached.
        let elapsed_time_since_start_of_motion = movement_start.elapsed().as_secs_f32();

        // if the current movement has been completed:
        if elapsed_time_since_start_of_motion > duration.as_secs_f32() {
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
        } else {
            nao_manager.set_all(
                lerp(
                    motion_manager.source_position.as_ref().unwrap(),
                    target_position,
                    elapsed_time_since_start_of_motion / duration.as_secs_f32(),
                ),
                JointArray::<f32>::fill(submotion_stiffness),
                Priority::High,
            );

            return Ok(());
        }
    }

    // set next joint positions
    match motion.get_position(
        sub_motion_name,
        &mut motion_manager.active_motion.as_mut().unwrap(),
    ) {
        Some(position) => {
            nao_manager.set_all(
                position,
                JointArray::<f32>::fill(submotion_stiffness),
                Priority::High,
            );
        }
        None => {
            if motion.submotions[sub_motion_name].exit_waittime > 0.05 {
                // firstly, we record the current timestamp and check whether the motion needs to wait
                match motion_manager.submotion_finishing_time {
                    None => {
                        motion_manager.submotion_finishing_time = Some(Instant::now());
                        return Ok(());
                    }

                    // checking whether the required waittime has elapsed
                    Some(finishing_time) => {
                        if finishing_time.elapsed().as_secs_f32()
                            < motion.submotions[sub_motion_name].exit_waittime
                        {
                            return Ok(());
                        }
                    }
                }
            }

            // current submotion is finished, transition to next submotion.
            let mut active_motion: ActiveMotion = motion_manager.get_active_motion().unwrap();

            motion_manager.submotion_execution_starting_time = None;
            motion_manager.submotion_finishing_time = None;
            motion_manager.source_position = None;

            match active_motion.get_next_submotion() {
                // If there is a next submotion, we attempt a transition
                Some(submotion_name) => {
                    motion_manager.active_motion =
                        active_motion.transition(nao_state, submotion_name);

                    // if the motion was aborted or an error occured with transitioning, we reset the execution time
                    if motion_manager.active_motion.is_none() {
                        motion_manager.motion_execution_starting_time = None;
                    }
                }
                None => {
                    // if no submotion is found, the motion has finished
                    motion_manager.active_motion = None;
                    motion_manager.motion_execution_starting_time = None;
                }
            }
        }
    }

    Ok(())
}
