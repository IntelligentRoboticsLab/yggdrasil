use super::{get_min_duration, lerp, types::Movement, ActiveMotion, MotionManager};
use crate::nao::manager::NaoManager;
use crate::nao::manager::Priority;
use crate::sensor::imu::IMUValues;
use crate::sensor::{falling::FallState, orientation::RobotOrientation};
use miette::{miette, Result};
use nalgebra::Vector3;
use nidhogg::types::{ArmJoints, ForceSensitiveResistors, HeadJoints, LegJoints};
use nidhogg::{
    types::{FillExt, JointArray},
    NaoState,
};
use std::time::{Duration, Instant};
use tyr::prelude::*;

// maximum speed the robot is allowed to move to the starting position at
const MAX_SPEED: f32 = 1.0;

// maximum gyroscopic value the robot can take for it to be considered steady
const MAX_GYRO_VALUE: f32 = 0.4;

// maximum accelerometer value the robot can take for it to be considered steady
const MAX_ACC_VALUE: f32 = 0.6;

// minimum fsr value the robot can take to be considered steady
const MIN_FSR_VALUE: f32 = 0.0;

// minimum waittime duration, anything less will not be considered
// (if we were to consider this waiting time, the amount of time to
// process it would take longer than the actual waittime)
const MINIMUM_WAIT_TIME: f32 = 0.05;

/// Executes the current motion.
///
/// # Arguments
/// * `nao_state` - State of the robot.
/// * `motion_manager` - Keeps track of state needed for playing motions.
/// * `nao_manager` - Used to set the new joint positions.
#[system]
pub fn motion_executer(
    nao_state: &mut NaoState,
    motion_manager: &mut MotionManager,
    nao_manager: &mut NaoManager,
    fall_state: &mut FallState,
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
            prepare_initial_movement(motion_manager, target_position, duration, &sub_motion_name)?;
        }

        // getting the next position for the robot
        if let Some(next_position) = move_to_starting_position(
            motion_manager,
            target_position,
            duration,
            &movement_start.elapsed(),
        ) {
            nao_manager.set_all(
                next_position,
                HeadJoints::<f32>::fill(submotion_stiffness),
                ArmJoints::<f32>::fill(submotion_stiffness),
                LegJoints::<f32>::fill(submotion_stiffness),
                Priority::High,
            );
            return Ok(());
        } else {
            // if the starting position has been reached,
            // we update the active motion for executing the submotion
            update_active_motion(motion_manager);
        }
    }

    // set next joint positions
    if let Some(position) = motion.get_position(
        &sub_motion_name,
        motion_manager.active_motion.as_mut().unwrap(),
    ) {
        nao_manager.set_all(
            position,
            HeadJoints::<f32>::fill(submotion_stiffness),
            ArmJoints::<f32>::fill(submotion_stiffness),
            LegJoints::<f32>::fill(submotion_stiffness),
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
            MAX_GYRO_VALUE,
            MAX_ACC_VALUE,
            MIN_FSR_VALUE,
        ) {
            // if not, we wait until it is either steady or the maximum wait time has elapsed
            if !exit_waittime_elapsed(
                motion_manager,
                motion.submotions[&sub_motion_name].exit_waittime,
            ) {
                // returning the current nao position to prohibit any other position requests from taking over
                nao_manager.set_all(
                    nao_state.position.clone(),
                    HeadJoints::<f32>::fill(submotion_stiffness),
                    ArmJoints::<f32>::fill(submotion_stiffness),
                    LegJoints::<f32>::fill(submotion_stiffness),
                    Priority::High,
                );
                return Ok(());
            }
        }

        transition_to_next_submotion(motion_manager, nao_state, fall_state).map_err(|err| {
            motion_manager.stop_motion();
            err
        })?;

        nao_manager.set_all(
            nao_state.position.clone(),
            HeadJoints::<f32>::fill(submotion_stiffness),
            ArmJoints::<f32>::fill(submotion_stiffness),
            LegJoints::<f32>::fill(submotion_stiffness),
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
) -> Result<()> {
    // checking whether the given duration will exceed our maximum speed limit
    let min_duration = get_min_duration(
        motion_manager
            .source_position
            .as_ref()
            .ok_or_else(|| miette!("Getting the source position failed during initial movement"))?,
        target_position,
        MAX_SPEED,
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

    Ok(())
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

/// Assesses whether the required waiting time has elapsed.
///
/// # Arguments
/// * `motion_manager` - Keeps track of state needed for playing motions.
/// * `duration` - Intended duration of the waiting time.
fn exit_waittime_elapsed(motion_manager: &mut MotionManager, exit_wait_time: f32) -> bool {
    if exit_wait_time <= MINIMUM_WAIT_TIME {
        return true;
    }

    // firstly, we record the current timestamp and check whether the motion needs to wait
    if let Some(finishing_time) = motion_manager.submotion_finishing_time {
        // checking whether the required waittime has elapsed
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
    fall_state: &mut FallState,
) -> Result<()> {
    // current submotion is finished, transition to next submotion.
    let active_motion: &mut ActiveMotion =
        motion_manager.active_motion.as_mut().ok_or_else(|| {
            miette!("No active motion present during transition, have you started a motion?")
        })?;

    motion_manager.submotion_execution_starting_time = None;
    motion_manager.submotion_finishing_time = None;
    motion_manager.source_position = None;

    if let Some(submotion_name) = active_motion.get_next_submotion() {
        // If there is a next submotion, we attempt a transition
        let next_submotion = active_motion.transition(nao_state, submotion_name.clone())?;
        motion_manager.active_motion = next_submotion;

        Ok(())
    }
    // if no submotion is found, the motion has finished
    else {
        // we send the appropriate exit message (if present)
        motion_manager
            .active_motion
            .as_ref()
            .unwrap()
            .execute_exit_routine(fall_state);

        // and we reset the Motionmanager
        motion_manager.active_motion = None;
        motion_manager.motion_execution_starting_time = None;

        Ok(())
    }
}
