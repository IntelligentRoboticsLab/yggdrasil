use std::time::Duration;

use miette::Result;
use nidhogg::{
    types::{FillExt, ForceSensitiveResistors, JointArray},
    NaoControlMessage,
};
use tyr::system;

use crate::{
    filter::{
        button::{ChestButton, HeadButtons},
        imu::IMUValues,
    },
    kinematics::FootOffset,
    primary_state::PrimaryState,
};

use super::{
    states::{self, WalkContext, WalkState, WalkStateKind},
    CycleTime, Odometry,
};

#[derive(Default, Clone)]
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

#[derive(Default)]
pub struct StepOffsets {
    pub swing: FootOffset,
    pub support: FootOffset,
}

#[derive(Default)]
pub struct WalkingEngine {
    state: WalkStateKind,
}

#[system]
pub fn toggle_walking_engine(
    head_button: &HeadButtons,
    chest_button: &ChestButton,
    walking_engine: &mut WalkingEngine,
) -> Result<()> {
    match (
        chest_button.state.is_pressed(),
        head_button.front.is_pressed(),
        &walking_engine.state,
    ) {
        // (true, false, WalkState::Idle { .. }) => {
        //     walking_engine.state = WalkState::Walking {
        //         walk_parameters: WalkCommand {
        //             forward: 0.04,
        //             left: 0.00,
        //             turn: 0.0,
        //             // turn: std::f32::consts::FRAC_PI_4,
        //         },
        //         swing_foot: Side::Left,
        //         phase_time: Duration::ZERO,
        //         filtered_gyro: Vector2::<f32>::default(),
        //         next_foot_switch: BASE_STEP_PERIOD,
        //         previous_step: StepOffsets::default(),
        //     };
        // }
        (true, false, WalkState::Idle) => {
            walking_engine.state = WalkState::Idle;
            // walking_engine.state = WalkState::Starting { hip_height: 0.10 };
        }
        (false, true, WalkState::_Starting { .. }) => {
            walking_engine.state = WalkState::Idle;
        }
        _ => (),
    };

    Ok(())
}

#[system]
pub fn walking_engine(
    walking_engine: &mut WalkingEngine,
    primary_state: &PrimaryState,
    cycle_time: &CycleTime,
    fsr: &ForceSensitiveResistors,
    imu: &IMUValues,
    control_message: &mut NaoControlMessage,
    odometry: &mut Odometry,
) -> Result<()> {
    // We don't run the walking engne whenever we're in unstiff.
    // This is a semi hacky way to prevent the robot from jumping up and
    // unstiffing itself when it's not supposed to.
    // We should definitely fix this in the future.
    if *primary_state == PrimaryState::Unstiff {
        // This sets the robot to be completely unstiff, completely disabling the joint motors.
        control_message.stiffness = JointArray::<f32>::fill(-1.0);
        return Ok(());
    }

    let mut context = WalkContext {
        walk_command: WalkCommand::default(),
        dt: cycle_time.duration,
        filtered_gyro: Default::default(),
        fsr: *fsr,
        imu: *imu,
        control_message
    };
    walking_engine.state = walking_engine.state.next_state(&context);

    

    // walking_engine.state = match &walking_engine.state {
    //     WalkState::Idle => {
    //         *odometry = Default::default();
    //         // control_message.stiffness = JointArray::<f32>::builder()
    //         //     .left_leg_joints(LeftLegJoints::fill(-1.0))
    //         //     .right_leg_joints(RightLegJoints::fill(-1.0))
    //         //     .build();
    //         states::idle_state(control_message)
    //     }
    //     WalkState::_Standing { .. } => todo!(),
    //     WalkState::_Starting { .. } => todo!(),
    //     WalkState::_Stopping => todo!(),
    //     WalkState::Walking {
    //         walk_parameters,
    //         swing_foot,
    //         phase_time,
    //         next_foot_switch,
    //         previous_step,
    //         filtered_gyro,
    //     } => states::walk_state(
    //         walk_parameters,
    //         swing_foot,
    //         *phase_time + dt,
    //         next_foot_switch,
    //         previous_step,
    //         filtered_gyro,
    //         fsr,
    //         imu,
    //         control_message,
    //         odometry,
    //     ),
    // };

    Ok(())
}
