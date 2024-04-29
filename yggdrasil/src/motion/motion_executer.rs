use crate::filter::{falling::FallState, imu::IMUValues, orientation::RobotOrientation};
use crate::motion::{
    motion_manager::{check_condition, ActiveMotion, MotionManager},
    motion_types::{InterpolationType, MotionType, Movement},
    motion_util::{get_min_duration, interpolate_jointarrays},
    MotionConfig,
};
use crate::nao::manager::{NaoManager, Priority};
use miette::{miette, Result};
use nalgebra::Vector3;
use nidhogg::{
    types::{ArmJoints, FillExt, ForceSensitiveResistors, HeadJoints, JointArray, LegJoints},
    NaoState,
};
use std::time::{Duration, Instant};
use tyr::prelude::*;

/// Executes the current motion.
///
/// # Arguments
/// * `nao_state` - State of the robot.
/// * `motion_manager` - Keeps track of state needed for playing motions.
/// * `nao_manager` - Used to set the new joint positions.
#[system]
#[allow(clippy::too_many_arguments)]
pub fn motion_executer(
    nao_state: &mut NaoState,
    motion_manager: &mut MotionManager,
    nao_manager: &mut NaoManager,
    fall_state: &mut FallState,
    config: &MotionConfig,
    orientation: &RobotOrientation,
    fsr: &ForceSensitiveResistors,
    imu: &IMUValues,
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
        movement_start,
        ..
    } = motion_manager.active_motion.clone().ok_or_else(|| {
        motion_manager.stop_motion();
        miette!("Motionmanager.ActiveMotion could not be cloned, likely contained None")
    })?;

    let current_submotion = &motion.submotions[&sub_motion_name];

    // checking the conditions set for catching a fall during the motion early
    // Note: Currently it only supports making the robot catch itself when falling,
    // it's possible to support more routines here but it does not seem logical to me.
    // Only real addition I would like to make is to support multiple catch types.
    if let Some(conditions) = &current_submotion.torso_angle_bounds {
        for condition in conditions {
            if !check_condition(nao_state, fsr, condition) {
                println!("{:?}", sub_motion_name);
                println!("Falling Again");
                // execute the appropriate catchfall motion depending on the direction of the fall
                if nao_state.angles.y > 0.0 {
                    motion_manager
                        .start_new_motion(MotionType::CatchFallForwards, Priority::Critical);
                } else {
                    motion_manager
                        .start_new_motion(MotionType::CatchFallBackwards, Priority::Critical);
                }

                return Ok(());
            }
        }
    }

    // at the start of a new submotion, we need to lerp to the starting position
    if motion_manager.submotion_execution_starting_time.is_none() {
        let Movement {
            target_position,
            duration,
            movement_interpolation_type,
        } = &motion.initial_movement(&sub_motion_name);

        // before beginning the first movement, we have to prepare the movement to avoid damage
        if motion_manager.position_buffer.is_none() {
            // record the last position before motion initialization, or before transition
            motion_manager.position_buffer = Some(nao_state.position.clone());
            prepare_initial_movement(
                motion_manager,
                target_position,
                duration,
                &sub_motion_name,
                config,
            );
        }

        // using the global interpolation type, unless the movement is assigned one already
        let initial_interpolation_type = movement_interpolation_type
            .as_ref()
            .or(Some(&motion.settings.global_interpolation_type))
            .ok_or_else(|| miette!("Problem with getting the global interpolation type"))?;

        // getting the next position for the robot
        if let Some(next_position) = move_to_starting_position(
            motion_manager,
            target_position,
            duration,
            &movement_start.elapsed(),
            initial_interpolation_type,
        ) {
            nao_manager.set_all(
                next_position,
                HeadJoints::<f32>::fill(current_submotion.joint_stifness),
                ArmJoints::<f32>::fill(current_submotion.joint_stifness),
                LegJoints::<f32>::fill(current_submotion.joint_stifness),
                Priority::High,
            );
            return Ok(());
        } else {
            // if the starting position has been reached,
            // we update the active motion for executing the submotion
            update_active_motion(motion_manager);

            // emptying the position buffer
            motion_manager.position_buffer = None;
        }
    }

    // set next joint positions
    if let Some(position) = motion.get_position(
        &sub_motion_name,
        motion_manager.active_motion.as_mut().unwrap(),
    )? {
        nao_manager.set_all(
            position,
            HeadJoints::<f32>::fill(current_submotion.joint_stifness),
            ArmJoints::<f32>::fill(current_submotion.joint_stifness),
            LegJoints::<f32>::fill(current_submotion.joint_stifness),
            Priority::High,
        );
    } else {
        let gyro = Vector3::new(imu.gyroscope.x, imu.gyroscope.y, imu.gyroscope.z);
        let linear_acceleration = Vector3::new(
            imu.accelerometer.x,
            imu.accelerometer.y,
            imu.accelerometer.z,
        );
        // we check whether the robot is in a steady position
        if !orientation.is_steady(
            gyro,
            linear_acceleration,
            fsr,
            config.max_stable_gyro_value,
            config.max_stable_acc_value,
            config.min_stable_fsr_value,
        ) {
            // if not, we wait until it is either steady or the maximum wait time has elapsed
            if !exit_wait_time_elapsed(motion_manager, current_submotion.exit_wait_time, config) {
                // since the current nao_state.position can vary during a standstill,
                // we simply put the first position in a buffer to reference to
                if motion_manager.position_buffer.is_none() {
                    motion_manager.position_buffer = Some(nao_state.position.clone());
                }

                // returning the current nao position to prohibit any other position requests from taking over
                nao_manager.set_all(
                    motion_manager.position_buffer.as_ref().unwrap().clone(),
                    HeadJoints::<f32>::fill(current_submotion.joint_stifness),
                    ArmJoints::<f32>::fill(current_submotion.joint_stifness),
                    LegJoints::<f32>::fill(current_submotion.joint_stifness),
                    Priority::High,
                );

                return Ok(());
            }
        }

        transition_to_next_submotion(motion_manager, nao_state, fsr, fall_state).map_err(
            |err| {
                motion_manager.stop_motion();
                err
            },
        )?;

        nao_manager.set_all(
            nao_state.position.clone(),
            HeadJoints::<f32>::fill(current_submotion.joint_stifness),
            ArmJoints::<f32>::fill(current_submotion.joint_stifness),
            LegJoints::<f32>::fill(current_submotion.joint_stifness),
            Priority::High,
        );
    }

    Ok(())
}

/// Prepares the initial movement of a submotion.
///
///
/// # Notes
/// Currently only checks and possibly edits the movement duration to prevent dangerously
/// quick movements, but will be expanded upon.
///
/// # Arguments
/// * `motion_manager` - Keeps track of state needed for playing motions.
/// * `target_position` - The target position of the initial movement.
/// * `duration` - Intended duration of the initial movement.
/// * `sub_motion_name` - Current submotion to be executed.
fn prepare_initial_movement(
    motion_manager: &mut MotionManager,
    target_position: &JointArray<f32>,
    duration: &Duration,
    sub_motion_name: &String,
    config: &MotionConfig,
) {
    // checking whether the given duration will exceed our maximum speed limit
    let min_duration = get_min_duration(
        motion_manager.position_buffer.as_ref().unwrap(),
        target_position,
        config.maximum_joint_speed,
    );
    if duration > &min_duration {
        // editing the movement duration to prevent dangerously quick movements
        motion_manager
            .active_motion
            .as_mut()
            .unwrap()
            .motion
            .set_initial_duration(sub_motion_name, min_duration);
    }
}

/// Updates the active motion to begin executing the current submotion.
///
/// # Arguments
/// * `motion_manager` - Keeps track of state needed for playing motions.
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

/// Calculates the next position of the robot to approach the starting position.
/// If the robot has reached the starting position, it will return None.
///
/// # Notes
/// Currently the function is still quite barren, but this will be expanded upon later.
/// For example, different interpolation types will be available.
///
/// # Arguments
/// * `motion_manager` - Keeps track of state needed for playing motions.
/// * `target_position` - The target position of the initial movement.
/// * `duration` - Intended duration of the initial movement.
/// * `elapsed_time` - Currently elapsed time since start of movement to initial position.
fn move_to_starting_position(
    motion_manager: &MotionManager,
    target_position: &JointArray<f32>,
    duration: &Duration,
    elapsed_time_since_start_of_motion: &Duration,
    interpolation_type: &InterpolationType,
) -> Option<JointArray<f32>> {
    if elapsed_time_since_start_of_motion <= duration {
        return Some(interpolate_jointarrays(
            motion_manager.position_buffer.as_ref().unwrap(),
            target_position,
            elapsed_time_since_start_of_motion.as_secs_f32() / duration.as_secs_f32(),
            interpolation_type,
        ));
    }

    None
}

/// Assesses whether the required waiting time has elapsed.
///
/// # Notes
/// Currently, the waiting time is static, with the robot always waiting
/// the full duration of the waiting time. But in the future this waiting time
/// might be shortened due to the robot being in a stable position.
///
/// # Arguments
/// * `motion_manager` - Keeps track of state needed for playing motions.
/// * `duration` - Intended duration of the waiting time.
fn exit_wait_time_elapsed(
    motion_manager: &mut MotionManager,
    exit_wait_time: f32,
    config: &MotionConfig,
) -> bool {
    if exit_wait_time <= config.minimum_wait_time {
        return true;
    }

    // firstly, we record the current timestamp and check whether the motion needs to wait
    if let Some(finishing_time) = motion_manager.submotion_finishing_time {
        // checking whether the required wait time has elapsed
        if finishing_time.elapsed().as_secs_f32() < exit_wait_time {
            return false;
        }

        true
    } else {
        motion_manager.submotion_finishing_time = Some(Instant::now());
        false
    }
}

/// Handles the logic for transitioning to the next submotion.
/// If a submotion is present, will transition to this submotion.
/// If not, will reset the active motion and saved time values.
///
/// # Notes
/// More complex transitioning behaviour will be implemented, like
/// having multiple movement paths the robot can decide to go in.
/// But this will be implemented far later.
///
/// # Arguments
/// * `motion_manager` - Keeps track of state needed for playing motions.
/// * `nao_state` - Current state of the robot.
fn transition_to_next_submotion(
    motion_manager: &mut MotionManager,
    nao_state: &mut NaoState,
    fsr: &ForceSensitiveResistors,
    fall_state: &mut FallState,
) -> Result<()> {
    // current submotion is finished, transition to next submotion.
    let active_motion: &mut ActiveMotion =
        motion_manager.active_motion.as_mut().ok_or_else(|| {
            miette!("No active motion present during transition, have you started a motion?")
        })?;

    motion_manager.submotion_execution_starting_time = None;
    motion_manager.submotion_finishing_time = None;
    motion_manager.position_buffer = None;

    let next_submotion: Option<ActiveMotion>;

    if let Some(submotion_name) = active_motion.get_next_submotion() {
        // If there is a next submotion, we attempt a transition
        next_submotion = active_motion.transition(nao_state, fsr, submotion_name.clone())?;
    } else {
        // if no submotion is found, the motion has finished
        // we execute the appropriate exit routine (if present)
        motion_manager
            .active_motion
            .as_ref()
            .unwrap()
            .execute_exit_routine(fall_state);

        // and we reset the MotionManager
        motion_manager.active_motion = None;
        motion_manager.motion_execution_starting_time = None;

        // we exit out of the transition, having finished the motion succesfully
        return Ok(());
    }

    // if a next submotion was found but the transition resulted in an abort,
    // we stop the motion without executing it's assigned succes routine
    if next_submotion.is_none() {
        // we simply stop the current Motion
        motion_manager.motion_execution_starting_time = None;
    }

    motion_manager.active_motion = next_submotion;
    Ok(())
}
