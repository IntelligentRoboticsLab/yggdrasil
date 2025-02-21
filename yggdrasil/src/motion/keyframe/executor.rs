use super::InterpolationType;
use super::{
    clamp_speed, interpolate_jointarrays, types::Movement, ActiveMotion, AnimationManager,
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

// standard priority for all animations
const ANIMATION_PRIORITY: Priority = Priority::High;

/// Executes the currently chosen motion as a simple animation.
///
/// # Arguments
/// * `nao_state` - State of the robot.
/// * `animation_manager` - Keeps track of state needed for playing animations.
/// * `nao_manager` - Used to set the new joint positions.
#[allow(clippy::too_many_lines)]
pub fn animation_executor(
    nao_state: ResMut<NaoState>,
    mut animation_manager: ResMut<AnimationManager>,
    mut nao_manager: ResMut<NaoManager>,
) {
    if animation_manager.active_motion.is_none() {
        return;
    }

    // keeping track of the moment that the current animation has started
    if animation_manager.motion_execution_starting_time.is_none() {
        animation_manager.motion_execution_starting_time = Some(Instant::now());
    }

    let active_motion = animation_manager
        .active_motion
        .clone()
        .ok_or_else(|| {
            animation_manager.stop_motion();
            warn!("animation_manager.active_motion could not be cloned, likely contained None")
        })
        .expect("failed to clone active motion");

    // setting up some variables that are used frequently
    let ActiveMotion {
        ref motion,
        cur_sub_motion: (ref sub_motion_name, _),
        ref movement_start,
        cur_keyframe_index: _,
    } = active_motion;

    let submotion_stiffness: f32 = motion.submotions[sub_motion_name].joint_stifness;

    // at the start of a new submotion, we need to lerp to the starting position
    if animation_manager
        .submotion_execution_starting_time
        .is_none()
    {
        let Movement {
            target_position,
            duration,
        } = active_motion.initial_movement(&sub_motion_name);

        // before beginning the first movement, we have to prepare the movement to avoid damage
        if animation_manager.source_position.is_none() {
            // record the last position before motion initialization, or before transition
            animation_manager.source_position = Some(nao_state.position.clone());

            prepare_movement(
                &mut animation_manager,
                target_position,
                duration,
                sub_motion_name,
            );
        }

        // getting the next position for the robot
        if let Some(next_position) = move_to_starting_position(
            &animation_manager,
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
                ANIMATION_PRIORITY,
            );
        } else {
            // if the starting position has been reached,
            // we update the active motion for executing the submotion
            update_active_motion(&mut animation_manager);
        }
    }

    // set next joint positions
    if let Some(position) = animation_manager
        .active_motion
        .as_mut()
        .unwrap()
        .get_position()
    {
        nao_manager.set_all(
            position,
            HeadJoints::<f32>::fill(submotion_stiffness),
            ArmJoints::<f32>::fill(submotion_stiffness),
            LegJoints::<f32>::fill(submotion_stiffness),
            ANIMATION_PRIORITY,
        );
    } else {
        transition_to_next_submotion(&mut animation_manager)
            .inspect_err(|_| animation_manager.stop_motion())
            .expect("failed to transition to next submotion");

        nao_manager.set_all(
            nao_state.position.clone(),
            HeadJoints::<f32>::fill(submotion_stiffness),
            ArmJoints::<f32>::fill(submotion_stiffness),
            LegJoints::<f32>::fill(submotion_stiffness),
            ANIMATION_PRIORITY,
        );
    }
}

/// Prepares the movement to prevent dangerous movement speeds, by clamping the movement speed to the maximum speed.
///
/// # Arguments
/// * `animation_manager` - A mutable reference to the `AnimationManager` which manages the active motion and its settings.
/// * `target_position` - The desired target position of the joints for the movement.
/// * `duration` - The initial duration of the movement.
/// * `sub_motion_name` - The name of the submotion to which the movement corresponds.
fn prepare_movement(
    animation_manager: &mut AnimationManager,
    target_position: &JointArray<f32>,
    duration: &Duration,
    sub_motion_name: &String,
) {
    if let Some(source_position) = &animation_manager.source_position {
        let motion_duration = clamp_speed(source_position, target_position, duration, &MAX_SPEED);

        // editing the movement duration in case it is too low to prevent dangerously quick movements
        animation_manager
            .active_motion
            .as_mut()
            .unwrap()
            .motion
            .set_initial_duration(&sub_motion_name, motion_duration);
    } else {
        error!("Getting the source position failed during initial movement");
        return;
    }
}

/// Updates the active motion to begin executing the current submotion.
///
/// # Arguments
/// * `animation_manager` - Keeps track of state needed for playing animations.
fn update_active_motion(animation_manager: &mut AnimationManager) {
    // update the time of the start of the movement
    animation_manager.submotion_execution_starting_time = Some(Instant::now());
    animation_manager
        .active_motion
        .as_mut()
        .unwrap()
        .movement_start = Instant::now();
    animation_manager
        .active_motion
        .as_mut()
        .unwrap()
        .cur_keyframe_index += 1;
}

/// Calculates the next position of the robot to approach the starting position.
/// If the robot has reached the starting position, it will return None.
///
/// # Arguments
/// * `animation_manager` - Keeps track of state needed for playing animations.
/// * `target_position` - The target position of the initial movement.
/// * `duration` - Intended duration of the initial movement.
/// * `elapsed_time` - Currently elapsed time since start of movement to initial position.
fn move_to_starting_position(
    animation_manager: &AnimationManager,
    target_position: &JointArray<f32>,
    duration: &Duration,
    elapsed_time_since_start_of_motion: &Duration,
    interpolation_type: &InterpolationType,
) -> Option<JointArray<f32>> {
    if elapsed_time_since_start_of_motion <= duration {
        return Some(interpolate_jointarrays(
            animation_manager.source_position.as_ref().unwrap(),
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
/// * `animation_manager` - Keeps track of state needed for playing animations.
fn transition_to_next_submotion(animation_manager: &mut AnimationManager) -> Result<()> {
    // current submotion is finished, transition to next submotion.
    let active_motion: &mut ActiveMotion =
        animation_manager.active_motion.as_mut().ok_or_else(|| {
            miette!("No active motion present during transition, have you started a motion?")
        })?;

    animation_manager.submotion_execution_starting_time = None;
    animation_manager.submotion_finishing_time = None;
    animation_manager.source_position = None;

    if let Some(submotion_name) = active_motion.get_next_submotion() {
        // If there is a next submotion, we attempt a transition
        active_motion.transition(submotion_name.clone());

        Ok(())
    }
    // if no submotion is found, the motion has finished
    else {
        // we reset the animation_manager
        animation_manager.active_motion = None;
        animation_manager.motion_execution_starting_time = None;

        Ok(())
    }
}
