use nidhogg::{
    types::{FillExt, ForceSensitiveResistors, JointArray},
    NaoControlMessage,
};

use crate::{
    filter::button::{ChestButton, HeadButtons},
    kinematics::FootOffset,
    prelude::*,
    primary_state::PrimaryState,
};

use super::{
    states::{self, WalkContext, WalkState, WalkStateKind},
    CycleTime, FilteredGyroscope,
};

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

#[derive(Default)]
pub struct WalkingEngine {
    state: WalkStateKind,
}

#[system]
pub fn toggle_walking_engine(
    primary_state: &PrimaryState,
    head_button: &HeadButtons,
    chest_button: &ChestButton,
    walking_engine: &mut WalkingEngine,
    filtered_gyro: &mut FilteredGyroscope,
) -> Result<()> {
    // If we're in a state where we shouldn't walk, we don't.
    if !primary_state.should_walk() {
        return Ok(());
    }

    match (
        chest_button.state.is_tapped(),
        head_button.front.is_tapped(),
        &walking_engine.state,
    ) {
        (true, false, WalkStateKind::Idle { .. }) => {
            filtered_gyro.reset();
            walking_engine.state = WalkStateKind::Walking(states::walking::WalkingState::default())
        }
        (false, true, WalkStateKind::Walking(states::walking::WalkingState { .. })) => {
            walking_engine.state = WalkStateKind::Idle(states::idle::IdleState { hip_height: 0.18 })
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
    filtered_gyro: &FilteredGyroscope,
    control_message: &mut NaoControlMessage,
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

    let mut context = WalkContext {
        walk_command: WalkCommand {
            forward: 0.1,
            left: 0.00,
            turn: 0.0,
        },
        dt: cycle_time.duration,
        filtered_gyro: filtered_gyro.0.clone(),
        fsr: fsr.clone(),
        control_message,
    };
    walking_engine.state = walking_engine.state.next_state(&mut context);

    Ok(())
}
