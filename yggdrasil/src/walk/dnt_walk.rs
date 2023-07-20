use std::time::Duration;

use miette::Result;
use nidhogg::{
    types::{
        FillExt, ForceSensitiveResistors, JointArray, LeftLegJoints, RightLegJoints, Vector2,
        Vector3,
    },
    NaoControlMessage,
};
use tyr::system;

use crate::filter::{
    button::{ChestButton, HeadButtons},
    imu::IMUValues,
};

use super::{
    kinematics::{self},
    CycleTime,
};

/// forward (the / by 4 is because the CoM moves as well and forwardL is wrt the CoM
const COM_MULTIPLIER: f32 = 0.25;

/// The base amount of time for one step, e.g. half a walk cycle.
const BASE_STEP_PERIOD: Duration = Duration::from_millis(240);

// the center of pressure threshold for switching support foot
const COP_PRESSURE_THRESHOLD: f32 = 0.2;

/// the base amount to lift a foot, in meters
const BASE_FOOT_LIFT: f32 = 0.01;

/// The hip height of the robot during the walking cycle
const HIP_HEIGHT: f32 = 0.185;

enum WalkState {
    Idle {
        hip_height: f32,
    },
    Starting,
    Stopping,
    Walking {
        walk_parameters: WalkCommand,
        swing_foot: Side,
        phase_time: Duration,
        next_foot_switch: Duration,
        previous_step: StepOffsets,
        filtered_gyro: Vector2<f32>,
    },
}

impl Default for WalkState {
    fn default() -> Self {
        Self::Idle {
            hip_height: HIP_HEIGHT,
        }
    }
}

#[derive(Default, Clone)]
struct WalkCommand {
    /// forward in meters per second
    forward: f32,
    /// side step in meters per second
    left: f32,
    /// turn in radians per second
    turn: f32,
}

#[derive(Debug, Default, Clone)]
enum Side {
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

#[derive(Default, Clone)]
pub struct FootOffset {
    pub forward: f32,
    pub left: f32,
    pub turn: f32,
    pub hip_height: f32,
    pub foot_lift: f32,
}

#[derive(Default)]
struct StepOffsets {
    pub swing: FootOffset,
    pub support: FootOffset,
}

#[derive(Default)]
pub struct WalkingEngine {
    state: WalkState,
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
        (true, false, WalkState::Idle { .. }) => {
            walking_engine.state = WalkState::Walking {
                walk_parameters: WalkCommand {
                    forward: 0.065,
                    left: 0.0,
                    turn: 0.0,
                },
                swing_foot: Side::Left,
                phase_time: Duration::ZERO,
                filtered_gyro: Vector2::<f32>::default(),
                next_foot_switch: BASE_STEP_PERIOD,
                previous_step: StepOffsets::default(),
            };
        }
        (false, true, WalkState::Walking { .. }) => {
            walking_engine.state = WalkState::Idle {
                hip_height: HIP_HEIGHT,
            };
        }
        _ => (),
    };

    Ok(())
}

#[system]
pub fn walking_engine(
    walking_engine: &mut WalkingEngine,
    cycle_time: &CycleTime,
    fsr: &ForceSensitiveResistors,
    imu: &IMUValues,
    control_message: &mut NaoControlMessage,
) -> Result<()> {
    let dt = cycle_time.duration;

    walking_engine.state = match &walking_engine.state {
        WalkState::Idle { hip_height } => idle_state(*hip_height, &mut control_message),
        WalkState::Starting => todo!(),
        WalkState::Stopping => todo!(),
        WalkState::Walking {
            walk_parameters,
            swing_foot,
            phase_time,
            next_foot_switch,
            previous_step,
            filtered_gyro,
        } => walk_state(
            walk_parameters,
            swing_foot,
            phase_time.clone() + dt,
            next_foot_switch,
            previous_step,
            filtered_gyro,
            &fsr,
            &imu,
            &mut control_message,
        ),
    };

    Ok(())
}

fn walk_state(
    walk_command: &WalkCommand,
    swing_foot: &Side,
    phase_time: Duration,
    next_foot_switch: &Duration,
    previous_step: &StepOffsets,
    filtered_gyro: &Vector2<f32>,
    fsr: &ForceSensitiveResistors,
    imu: &IMUValues,
    control_message: &mut NaoControlMessage,
) -> WalkState {
    // let's figure out the parameters for the walk in this current cycle
    let WalkCommand {
        forward,
        left,
        turn: _,
    } = walk_command;

    // this is the linear progression of this step, a value from 0 to 1 which describes the progress of the current step.
    let linear_time = (phase_time.as_secs_f32() / next_foot_switch.as_secs_f32()).clamp(0.0, 1.0);

    if next_foot_switch.as_secs_f32() <= 0.0 {
        return next_walk_state(
            linear_time,
            walk_command,
            swing_foot,
            &phase_time,
            next_foot_switch,
            previous_step.swing.clone(),
            previous_step.support.clone(),
            filtered_gyro,
            fsr,
        );
    }

    // compute the max foot height, for moving forward/left we slightly increase the max height
    let max_foot_height = BASE_FOOT_LIFT + (forward.abs() * 0.01) + (left.abs() * 0.02);
    // compute the swing foot height for the current cycle in the step phase
    let swing_foot_height = max_foot_height * parabolic_return(linear_time);

    // compute the offsets for the support and swing feet
    let support_offset = compute_support_offset(walk_command, linear_time, &previous_step.support);
    let swing_offset = compute_swing_offset(
        walk_command,
        swing_foot_height,
        linear_time,
        &previous_step.swing,
    );

    println!(
        "walk: {:?} forward: {}: left_pressure: {}, right_pressure: {}",
        swing_foot,
        swing_offset.forward,
        fsr.left_foot.sum(),
        fsr.right_foot.sum()
    );

    // sagittal/coronal balancing
    let filtered_gyro = filter_gyro_values(&filtered_gyro, &imu.gyroscope);

    let next_state = next_walk_state(
        linear_time,
        walk_command,
        swing_foot,
        &phase_time,
        &next_foot_switch,
        swing_offset.clone(),
        support_offset.clone(),
        &filtered_gyro,
        fsr,
    );

    let (left_foot, right_foot) = match swing_foot {
        Side::Left => (swing_offset, support_offset),
        Side::Right => (support_offset, swing_offset),
    };

    // the shoulder pitch is "approximated" by taking the opposite direction * 6
    // this results in a swing motion that moves in the opposite direction as the foot.
    let left_shoulder_pitch = -left_foot.forward * 6.0;
    let right_shoulder_pitch = -right_foot.forward * 6.0;

    let (_possible, mut left_leg_joints, mut right_leg_joints) =
        kinematics::hulk_ik::leg_angles(left_foot, right_foot);

    // let mut left_leg_joints = kinematics::inverse::left_leg_angles(&left_foot);
    // let mut right_leg_joints = kinematics::inverse::right_leg_angles(&right_foot);

    // balance adjustment
    let balance_adjustment = filtered_gyro.y / 25.0;
    if next_foot_switch.as_secs_f32() > 0.0 {
        match swing_foot {
            Side::Left => {
                right_leg_joints.ankle_pitch += balance_adjustment;
            }
            Side::Right => {
                left_leg_joints.ankle_pitch += balance_adjustment;
            }
        }
    } else {
        right_leg_joints.ankle_pitch += balance_adjustment;
        left_leg_joints.ankle_pitch += balance_adjustment;
    }

    control_message.stiffness = JointArray::<f32>::builder()
        .left_shoulder_pitch(1.0)
        .left_shoulder_roll(1.0)
        .right_shoulder_pitch(1.0)
        .right_shoulder_roll(1.0)
        .left_leg_joints(LeftLegJoints::fill(1.0))
        .right_leg_joints(RightLegJoints::fill(1.0))
        .build();

    control_message.position = JointArray::<f32>::builder()
        .left_shoulder_pitch(90f32.to_radians() + left_shoulder_pitch)
        .left_shoulder_roll(7f32.to_radians())
        .right_shoulder_pitch(90f32.to_radians() + right_shoulder_pitch)
        .right_shoulder_roll(-7f32.to_radians())
        .left_leg_joints(left_leg_joints)
        .right_leg_joints(right_leg_joints)
        .build();

    next_state
}

fn next_walk_state(
    linear_time: f32,
    walk_command: &WalkCommand,
    swing_foot: &Side,
    phase_time: &Duration,
    next_foot_switch: &Duration,
    swing_offset: FootOffset,
    support_offset: FootOffset,
    filtered_gyro: &Vector2<f32>,
    fsr: &ForceSensitiveResistors,
) -> WalkState {
    let mut next_swing_foot = swing_foot.clone();
    let mut phase_time = phase_time.clone();
    let mut next_foot_switch = next_foot_switch.clone();

    let mut previous_step = StepOffsets {
        swing: swing_offset.clone(),
        support: support_offset.clone(),
    };
    // figure out whether the support foot has changed
    let has_support_foot_changed = linear_time > 0.75 && has_support_foot_changed(&swing_foot, fsr);

    // if the support foot has in fact changed, we should update the relevant parameters
    if has_support_foot_changed {
        next_swing_foot = swing_foot.next();

        // reset phase
        next_foot_switch = BASE_STEP_PERIOD;
        phase_time = Duration::ZERO;

        // Switch these around, so the offsets are maintained throughout the walk cycle
        previous_step.support = swing_offset;
        previous_step.swing = support_offset;

        // TODO: switch left parameter to the value of swing_offset
    }

    WalkState::Walking {
        walk_parameters: walk_command.clone(),
        swing_foot: next_swing_foot,
        phase_time: phase_time,
        next_foot_switch: next_foot_switch,
        filtered_gyro: filtered_gyro.clone(),
        previous_step,
    }
}

fn filter_gyro_values(filtered_gyro: &Vector2<f32>, gyroscope: &Vector3<f32>) -> Vector2<f32> {
    Vector2 {
        x: 0.8 * filtered_gyro.x + 0.2 * gyroscope.x,
        y: 0.8 * filtered_gyro.y + 0.2 * gyroscope.y,
    }
}

fn has_support_foot_changed(side: &Side, fsr: &ForceSensitiveResistors) -> bool {
    let left_foot_pressure = fsr.left_foot.sum();
    let right_foot_pressure = fsr.right_foot.sum();
    (match side {
        Side::Left => left_foot_pressure,
        Side::Right => right_foot_pressure,
    }) > COP_PRESSURE_THRESHOLD
}

fn parabolic_return(time: f32) -> f32 {
    0.5 * (2.0 * std::f32::consts::PI * time - std::f32::consts::FRAC_PI_2).sin() + 0.5
}

pub fn parabolic_step(linear_time: f32) -> f32 {
    if linear_time < 0.5 {
        2.0 * linear_time.powi(2)
    } else {
        4.0 * linear_time - 2.0 * linear_time.powi(2) - 1.0
    }
}

fn idle_state(hip_height: f32, control_message: &mut NaoControlMessage) -> WalkState {
    let foot_position = FootOffset {
        forward: 0.0,
        left: 0.0,
        turn: 0.0,
        hip_height,
        foot_lift: 0.0,
    };

    let left_legs = kinematics::inverse::left_leg_angles(&foot_position);
    let right_legs = kinematics::inverse::right_leg_angles(&foot_position);

    control_message.stiffness = JointArray::<f32>::builder()
        .left_leg_joints(LeftLegJoints::fill(1.0))
        .right_leg_joints(RightLegJoints::fill(1.0))
        .build();

    control_message.position = JointArray::builder()
        .left_leg_joints(left_legs)
        .right_leg_joints(right_legs)
        .build();

    WalkState::Idle {
        hip_height: hip_height,
    }
}

fn compute_swing_offset(
    walk_command: &WalkCommand,
    foot_height: f32,
    linear_time: f32,
    step_t0: &FootOffset,
) -> FootOffset {
    let forward_t0 = step_t0.forward;
    let parabolic_time = parabolic_step(linear_time);
    FootOffset {
        forward: forward_t0 + (walk_command.forward * COM_MULTIPLIER - forward_t0) * parabolic_time,
        left: 0.0,
        turn: 0.0,
        hip_height: HIP_HEIGHT,
        foot_lift: foot_height,
    }
}

fn compute_support_offset(
    walk_command: &WalkCommand,
    linear_time: f32,
    step_t0: &FootOffset,
) -> FootOffset {
    let forward_t0 = step_t0.forward;
    FootOffset {
        forward: forward_t0 + (-walk_command.forward * COM_MULTIPLIER - forward_t0) * linear_time,
        left: 0.0,
        turn: 0.0,
        hip_height: HIP_HEIGHT,
        foot_lift: 0.0,
    }
}
