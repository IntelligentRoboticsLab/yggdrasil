use crate::motion::motion_manager::{ActiveMotion, MotionManager};
use crate::motion::motion_types::Movement;
use crate::motion::motion_util::lerp;
use miette::Result;
use nidhogg::{
    types::{FillExt, JointArray},
    NaoControlMessage, NaoState,
};
use std::time::Instant;
use tyr::prelude::*;

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

    // keeping track of the moment that the current motion has started
    if motion_manager.motion_execution_starting_time.is_none() {
        motion_manager.motion_execution_starting_time = Some(Instant::now());
    }

    // setting up some variables that are used frequently
    let ActiveMotion {
        motion,
        cur_sub_motion,
        cur_keyframe_index,
        movement_start,
    } = motion_manager.get_active_motion().unwrap();

    println!(
        "Executing: {}  Index: {}",
        cur_sub_motion.0, cur_keyframe_index
    );
    println!(
        "Movement Duration: {}\n",
        movement_start.elapsed().as_secs_f32()
    );

    let sub_motion_name: &String = &cur_sub_motion.0;
    let submotion_stiffness: f32 = motion.submotions[sub_motion_name].joint_stifness;

    // TODO implement maximum speed check here

    // at the start of a new submotion, we need to lerp to the starting position
    if motion_manager.submotion_execution_starting_time.is_none() {
        // record the last position before motion initialization, or before transition
        if motion_manager.source_position.is_none() {
            motion_manager.source_position = Some(nao_state.position.clone());
        }
        let Movement {
            target_position,
            duration,
        } = &motion.initial_movement(sub_motion_name);

        // Starting position has not yet been reached, so lerp to start
        // position, until position has been reached.
        let elapsed_time_since_start_of_motion: f32 = movement_start.elapsed().as_secs_f32();

        // if the current movement has been completed:
        if elapsed_time_since_start_of_motion > duration.as_secs_f32() {
            // update the time of the start of the movement
            println!("First Position Reached");

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
            nao_control_message.position = lerp(
                motion_manager.source_position.as_ref().unwrap(),
                target_position,
                elapsed_time_since_start_of_motion / duration.as_secs_f32(),
            );
            nao_control_message.stiffness = JointArray::<f32>::fill(submotion_stiffness);

            return Ok(());
        }
    }

    // set next joint positions
    match motion.get_position(
        sub_motion_name,
        &mut motion_manager.active_motion.as_mut().unwrap(),
    ) {
        Some(position) => {
            nao_control_message.position = position;
            nao_control_message.stiffness = JointArray::<f32>::fill(submotion_stiffness);
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
                        println!(
                            "Waiting: {} -> {}",
                            finishing_time.elapsed().as_secs_f32(),
                            motion.submotions[sub_motion_name].exit_waittime
                        );
                        if finishing_time.elapsed().as_secs_f32()
                            < motion.submotions[sub_motion_name].exit_waittime
                        {
                            return Ok(());
                        }
                    }
                }
            }
            println!("Done");

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
