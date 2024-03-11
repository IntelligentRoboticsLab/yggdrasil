use crate::motion::motion_manager::{ActiveMotion, MotionManager};
use crate::motion::motion_util::{lerp, reached_position};
use miette::Result;
use nidhogg::{
    types::{FillExt, JointArray},
    NaoControlMessage, NaoState,
};
use std::time::SystemTime;
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

    let ActiveMotion {
        motion,
        current_sub_motion,
        mut prev_keyframe_index,
        mut movement_start,
        starting_time,
    } = motion_manager.get_active_motion().unwrap();

    if motion_manager.motion_execution_starting_time.is_none() {
        if !reached_position(
            &nao_state.position,
            &motion.initial_movement().target_position,
            STARTING_POSITION_ERROR_MARGIN,
        ) {
            println!("Not reached starting position");
            // Starting position has not yet been reached, so lerp to start
            // position, until position has been reached.
            let elapsed_time_since_start_of_motion: f32 =
                starting_time.elapsed().unwrap().as_secs_f32();

            nao_control_message.position = lerp(
                &nao_state.position,
                &motion.initial_movement().target_position,
                elapsed_time_since_start_of_motion
                    / &motion.initial_movement().duration.as_secs_f32(),
            );
            nao_control_message.stiffness =
                JointArray::<f32>::fill(motion.submotions[&current_sub_motion.0].joint_stifness);

            return Ok(());
        } else {
            println!("Reached starting position");
            motion_manager.motion_execution_starting_time = Some(SystemTime::now());
        }
    }

    // set next joint positions
    match motion.get_position(
        &current_sub_motion.0,
        &mut prev_keyframe_index,
        &mut movement_start,
    ) {
        Some(position) => {
            nao_control_message.position = position;
            nao_control_message.stiffness =
                JointArray::<f32>::fill(motion.submotions[&current_sub_motion.0].joint_stifness);
        }
        None => {
            //Current submotion is finished, transition to next submotion.
            let active_motion = motion_manager.get_active_motion().unwrap();

            match active_motion.get_next_submotion() {
                // If there is a next submotion, we attempt a transition
                Some(submotion_name) => {
                    motion_manager.active_motion =
                        Some(active_motion.transition(nao_state, submotion_name))
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
