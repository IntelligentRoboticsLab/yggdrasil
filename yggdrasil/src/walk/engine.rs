use std::time::Duration;

use nidhogg::{
    types::{FillExt, ForceSensitiveResistors, JointArray, LeftLegJoints, RightLegJoints},
    NaoControlMessage,
};

use crate::{
    debug::DebugContext,
    filter::button::{ChestButton, HeadButtons},
    kinematics::FootOffset,
    prelude::*,
    primary_state::PrimaryState,
};

use super::{
    states::{self, WalkContext, WalkState, WalkStateKind},
    FilteredGyroscope, WalkingEngineConfig,
};
use crate::nao::CycleTime;

#[derive(Default, Clone, Debug)]
pub struct WalkCommand {
    /// forward in meters per second
    pub forward: f32,
    /// side step in meters per second
    pub left: f32,
    /// turn in radians per second
    pub turn: f32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Side {
    #[default]
    Left,
    Right,
}

impl Side {
    pub fn next(&self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct StepOffsets {
    pub swing: FootOffset,
    pub support: FootOffset,
}

pub struct WalkingEngine {
    pub state: WalkStateKind,
}

impl WalkingEngine {
    pub fn new(config: &WalkingEngineConfig) -> Self {
        Self {
            state: WalkStateKind::Idle(states::idle::IdleState::new(config)),
        }
    }
}

#[system]
pub fn toggle_walking_engine(
    primary_state: &PrimaryState,
    head_button: &HeadButtons,
    chest_button: &ChestButton,
    walking_config: &WalkingEngineConfig,
    walking_engine: &mut WalkingEngine,
    filtered_gyro: &mut FilteredGyroscope,
) -> Result<()> {
    // If we're in a state where we shouldn't walk, we don't.
    if !primary_state.should_walk() {
        return Ok(());
    }

    // Start walking
    if chest_button.state.is_tapped() {
        filtered_gyro.reset();
        let (swing, support) = walking_engine.state.get_foot_offsets();
        walking_engine.state = WalkStateKind::Walking(states::walking::WalkingState::new(
            walking_config,
            swing,
            support,
        ));
        return Ok(());
    }
    // Stop walking
    if head_button.front.is_tapped() {
        walking_engine.state = WalkStateKind::Idle(states::idle::IdleState {
            hip_height: walking_config.hip_height,
        });
        return Ok(());
    }

    Ok(())
}

#[system]
pub fn walking_engine(
    walking_engine: &mut WalkingEngine,
    config: &WalkingEngineConfig,
    primary_state: &PrimaryState,
    cycle_time: &CycleTime,
    fsr: &ForceSensitiveResistors,
    filtered_gyro: &FilteredGyroscope,
    control_message: &mut NaoControlMessage,
    dbg: &DebugContext,
) -> Result<()> {
    // We don't run the walking engine whenever we're in a state where we shouldn't.
    // This is a semi hacky way to prevent the robot from jumping up and
    // unstiffing itself when it's not supposed to.
    // TODO: We should definitely fix this in the future.
    if !primary_state.should_walk() {
        // This sets the robot to be completely unstiff, completely disabling the joint motors.
        control_message.stiffness = JointArray::<f32>::fill(-1.0);
        return Ok(());
    }

    let context = WalkContext {
        walk_command: WalkCommand {
            forward: 0.0,
            left: 0.05,
            turn: 0.0,
        },
        config,
        dt: cycle_time.duration,
        filtered_gyro,
        fsr,
        dbg: dbg.clone(),
    };

    walking_engine.state = walking_engine.state.clone().next_state(context);
    let (left_foot, right_foot) = walking_engine.state.get_foot_offsets();

    dbg.log_scalar_f32("/foot/left/forward", left_foot.forward)?;
    dbg.log_scalar_f32("/foot/left/lift", left_foot.lift)?;

    dbg.log_scalar_f32("/foot/right/forward", right_foot.forward)?;
    dbg.log_scalar_f32("/foot/right/lift", right_foot.lift)?;

    // set the stiffness and position of the legs
    let (mut left_leg_joints, mut right_leg_joints) =
        crate::kinematics::inverse::leg_angles(&left_foot, &right_foot);

    // balancing
    let swing = walking_engine.state.swing_foot();

    // the shoulder pitch is "approximated" by taking the opposite direction multiplied by a constant.
    // this results in a swing motion that moves in the opposite direction as the foot.
    let balancing_config = &config.balancing;
    let left_shoulder_pitch = -left_foot.forward * balancing_config.arm_swing_multiplier;
    let right_shoulder_pitch = -right_foot.forward * balancing_config.arm_swing_multiplier;

    // Balance adjustment
    let balance_adjustment = filtered_gyro.y() * balancing_config.filtered_gyro_y_multiplier;
    match swing {
        Side::Left => {
            right_leg_joints.ankle_pitch += balance_adjustment;
        }
        Side::Right => {
            left_leg_joints.ankle_pitch += balance_adjustment;
        }
    }

    control_message.position = JointArray::<f32>::builder()
        .left_shoulder_pitch(90f32.to_radians() + left_shoulder_pitch)
        .left_shoulder_roll(7f32.to_radians())
        .right_shoulder_pitch(90f32.to_radians() + right_shoulder_pitch)
        .right_shoulder_roll(-7f32.to_radians())
        .left_leg_joints(left_leg_joints)
        .right_leg_joints(right_leg_joints)
        .build();

    let stiffness = 1.0;

    control_message.stiffness = JointArray::<f32>::builder()
        .left_shoulder_pitch(stiffness)
        .left_shoulder_roll(stiffness)
        .right_shoulder_pitch(stiffness)
        .right_shoulder_roll(stiffness)
        .head_pitch(stiffness)
        .head_yaw(stiffness)
        .left_leg_joints(LeftLegJoints::fill(stiffness))
        .right_leg_joints(RightLegJoints::fill(stiffness))
        .build();

    Ok(())
}
