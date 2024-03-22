use crate::motion::motion_manager::{ActiveMotion, MotionManager};
use crate::motion::motion_util::{lerp, reached_position};
use miette::Result;
use nidhogg::{
    types::{FillExt, JointArray},
    NaoControlMessage, NaoState,
};
use std::time::Instant;
use tyr::prelude::*;

const STARTING_POSITION_ERROR_MARGIN: f32 = 0.40;

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
        prev_keyframe_index: _,
        movement_start,
    } = motion_manager.get_active_motion().unwrap();

    let sub_motion_index: &String = &cur_sub_motion.0;
    let submotion_stiffness: f32 = motion.submotions[sub_motion_index].joint_stifness;

    // at the start of a new submotion, we need to lerp to the starting position
    if motion_manager.submotion_execution_starting_time.is_none() {
        if !reached_position(
            &nao_state.position,
            &motion.initial_movement(sub_motion_index).target_position,
            STARTING_POSITION_ERROR_MARGIN,
        ) {
            println!("Not reached starting position");
            println!(
                "motion_execution_starting_time: {:?}",
                motion_manager.motion_execution_starting_time.unwrap()
            );
            println!("nao_state.position: {:?}", nao_state.position);
            println!(
                "target_position: {:?}",
                motion.initial_movement(sub_motion_index).target_position
            );
            // Starting position has not yet been reached, so lerp to start
            // position, until position has been reached.
            let elapsed_time_since_start_of_motion: f32 = movement_start.elapsed().as_secs_f32();

            println!(
                "elapsed_time_since_start_of_motion: {:?}",
                elapsed_time_since_start_of_motion
            );
            nao_control_message.position = lerp(
                &nao_state.position,
                &motion.initial_movement(sub_motion_index).target_position,
                elapsed_time_since_start_of_motion
                    / &motion
                        .initial_movement(sub_motion_index)
                        .duration
                        .as_secs_f32(),
            );
            nao_control_message.stiffness = JointArray::<f32>::fill(submotion_stiffness);

            return Ok(());
        } else {
            println!("Reached starting position");
            motion_manager.submotion_execution_starting_time = Some(Instant::now());
        }
    }

    // set next joint positions
    match motion.get_position(
        sub_motion_index,
        &mut motion_manager.active_motion.as_mut().unwrap(),
    ) {
        Some(position) => {
            nao_control_message.position = position;
            nao_control_message.stiffness = JointArray::<f32>::fill(submotion_stiffness);
        }
        None => {
            // current submotion is finished, transition to next submotion.
            let mut active_motion: ActiveMotion = motion_manager.get_active_motion().unwrap();

            match active_motion.get_next_submotion() {
                // If there is a next submotion, we attempt a transition
                Some(submotion_name) => {
                    motion_manager.active_motion =
                        active_motion.transition(nao_state, submotion_name)
                }
                None => {
                    // if no submotion is found, the motion has finished
                    motion_manager.active_motion = None;
                    motion_manager.motion_execution_starting_time = None;
                    motion_manager.submotion_execution_starting_time = None;
                }
            }
        }
    }

    Ok(())
}
