use std::{time::Duration};

use miette::Result;
use nidhogg::{
    types::{
        Color, FillExt, ForceSensitiveResistors, JointArray, LeftEye, LeftLegJoints,
        RightLegJoints, Vector2, Vector3,
    },
    NaoControlMessage,
};

use tracing::info;
use tyr::system;

use crate::filter::{
    button::{ChestButton, HeadButtons},
    imu::IMUValues,
};

use super::{
    kinematics::{self, inverse::FootPosition, robot_dimensions::ANKLE_TO_SOLE},
    CycleTime, dnt_walk::FootOffset,
};

#[derive(Default, PartialEq)]
pub enum WalkState {
    #[default]
    Stand,
    Walk,
}

const STEP_DURATION: Duration = Duration::from_millis(245);
const COP_PRESSURE_THRESHOLD: f32 = 0.2;

/// the base amount to lift a foot, in meters
const BASE_FOOT_LIFT: f32 = 0.01;

/// The hip height of the robot during the walking cycle
const HIP_HEIGHT: f32 = 0.185;

/// walk command
#[derive(Default)]
pub struct WalkCommand {
    /// forward in meters per second
    pub forward: f32,
    /// side step in meters per second
    pub left: f32,
    /// turn in radians per second
    pub turn: f32,
}

#[derive(Debug, Default)]
pub enum Side {
    #[default]
    Left,
    Right,
}

impl Side {
    pub fn is_left(&self) -> bool {
        match self {
            Side::Left => true,
            Side::Right => false,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

#[derive(Default, Debug)]
pub struct StepParameters {
    pub forward_l: f32,
    pub forward_r: f32,
    pub left_l: f32,
    pub left_r: f32,
}

#[derive(Default)]
pub struct WalkingEngine {
    pub state: WalkState,
    pub command: WalkCommand,
    pub phase_time: Duration,
    pub next_foot_switch: Duration,
    pub has_support_foot_changed: bool,
    pub swing_side: Side,
    pub last: StepParameters,
    pub filtered_gyro: Vector2<f32>,
}

#[system]
pub fn toggle_walking_engine(
    head_button: &HeadButtons,
    chest_button: &ChestButton,
    walking_engine: &mut WalkingEngine,
) -> Result<()> {
    if chest_button.state.is_pressed() && walking_engine.state == WalkState::Stand {
        walking_engine.state = WalkState::Walk;
        walking_engine.phase_time = Duration::ZERO;
        walking_engine.next_foot_switch = STEP_DURATION;
    }

    if head_button.front.is_pressed() && walking_engine.state == WalkState::Walk {
        walking_engine.state = WalkState::Stand;
    }

    Ok(())
}

#[system]
pub fn walking_engine(
    walking_engine: &mut WalkingEngine,
    imu_data: &IMUValues,
    cycle_time: &mut CycleTime,
    control_message: &mut NaoControlMessage,
    fsr: &ForceSensitiveResistors,
) -> Result<()> {
    walking_engine.command = WalkCommand {
        forward: 0.10,
        left: 0.00,
        turn: 0.0,
    };

    let color = match walking_engine.state {
        WalkState::Stand => Color::new(1.0, 0.0, 0.0),
        WalkState::Walk => Color::new(0.0, 1.0, 0.0),
    };

    control_message.left_eye = LeftEye::fill(color);

    walking_engine.phase_time += cycle_time.duration;
    let time = walking_engine.phase_time.as_secs_f32();
    let next_foot_switch = walking_engine.next_foot_switch.as_secs_f32();
    let linear_time = (walking_engine.phase_time.as_secs_f32() / next_foot_switch).clamp(0.0, 1.0);

    let stiffness = match walking_engine.state {
        WalkState::Stand => 1.0,
        WalkState::Walk => 1.0,
    };

    control_message.stiffness = JointArray::<f32>::builder()
        .left_shoulder_pitch(1.0)
        .right_shoulder_pitch(1.0)
        .left_leg_joints(LeftLegJoints::fill(stiffness))
        .right_leg_joints(RightLegJoints::fill(stiffness))
        .build();

    if walking_engine.state == WalkState::Stand {
        let joints = kinematics::inverse::left_leg_angles(&FootOffset {
            forward: 0.0,
            left: 0.0,
            turn: 0.0,
            hip_height: 0.18,
            foot_lift: 0.0,
        });
        control_message.position = JointArray::<f32>::builder()
            .left_shoulder_pitch(90f32.to_radians())
            .right_shoulder_pitch(90f32.to_radians())
            .left_leg_joints(joints.clone())
            .right_hip_pitch(joints.hip_pitch)
            .right_hip_roll(joints.hip_roll)
            .right_knee_pitch(joints.knee_pitch)
            .right_ankle_pitch(joints.ankle_pitch)
            .right_ankle_roll(joints.ankle_roll)
            .build();

        return Ok(());
    }

    let WalkCommand {
        forward,
        left,
        turn: _,
    } = walking_engine.command;

    if walking_engine.state == WalkState::Walk && next_foot_switch > 0.0 {
        let max_foot_height = BASE_FOOT_LIFT + (forward.abs() * 0.01) + (left.abs() * 0.02);
        let var_leg_height = max_foot_height * parabolic_return(linear_time);

        let StepParameters {
            forward_l: forward_l0,
            forward_r: forward_r0,
            left_l: left_l0,
            left_r: left_r0,
        } = walking_engine.last;
        let mut forward_l = 0_f32;
        let mut forward_r = 0_f32;
        let mut left_r = 0_f32;
        let mut left_l = 0_f32;

        let mut foot_height_l = 0.0;
        let mut foot_height_r = 0.0;
        let meth = 3.0;
        if walking_engine.swing_side.is_left() {
            forward_r =
                forward_r0 + (-forward / meth - forward_r0) * linear_time;
            forward_l = forward_l0
                + linear_time
                    * (forward / meth - forward_l0);

            if left > 0_f32 {
                // share angle between both feet, hence / 2
                left_r = left_angle(left, HIP_HEIGHT, linear_time) / 2.0;
                left_l = -left_r;
            } else {
                // recover from previous angle
                left_l = left_l0 * (1.0 - parabolic_step(linear_time));
                left_r = -left_l;
            }

            foot_height_l = var_leg_height;
            foot_height_r = 0.0;
        } else {
            forward_l =
                forward_l0 + (-forward / meth - forward_l0) * linear_time;
            forward_r = forward_r0
                + linear_time
                    * (forward / meth - forward_r0);

            if left > 0_f32 {
                // share angle between both feet, hence / 2
                left_l = left_angle(left, HIP_HEIGHT, linear_time) / 2.0;
                left_r = -left_l;
            } else {
                // recover from previous angle
                left_r = left_r0 * (1.0 - parabolic_step(linear_time));
                left_l = -left_r;
            }

            foot_height_r = var_leg_height;
            foot_height_l = 0.0;
        }

        let shoulder_pitch_r = -forward_r * 6.0;
        let shoulder_pitch_l = -forward_l * 6.0;

        let has_support_foot_changed =
            linear_time > 0.75 && has_support_foot_changed(&walking_engine.swing_side, &fsr);
        // info!("[{}] {:?} ({}) == {}: forward_l: {forward_l}, forward_r: {forward_r}, last_l: {}, last_r: {}", dt, walking_engine.side, phase, has_support_foot_changed, walking_engine.last.forward_l, walking_engine.last.forward_r);
        if has_support_foot_changed {
            walking_engine.swing_side = walking_engine.swing_side.next();

            // reset phase
            walking_engine.next_foot_switch = STEP_DURATION;
            walking_engine.phase_time = Duration::ZERO;

            // update last step parameters
            walking_engine.last.forward_l = forward_l;
            walking_engine.last.forward_r = forward_r;
            match walking_engine.swing_side {
                Side::Left => {
                    walking_engine.last.left_l = left_l;
                    walking_engine.last.left_r = left_l;
                }
                Side::Right => {
                    walking_engine.last.left_l = left_r;
                    walking_engine.last.left_r = left_r;
                }
            }
        }

        // sagittal/coronal balancing
        walking_engine.filtered_gyro =
            filter_gyro_values(&walking_engine.filtered_gyro, &imu_data.gyroscope);

        // compute joint angles
        let og_left_leg_joints = kinematics::inverse::left_leg_angles(&FootOffset {
            forward: forward_l,
            left: left_l,
            turn: 0.0,
            hip_height: HIP_HEIGHT,
            foot_lift: foot_height_l,
        });

        info!("[OG] time: {}, phase: {}, parabolic_step: {}, parabolic_return: {}", time, linear_time, parabolic_step(linear_time), parabolic_return(linear_time));
        info!("[OG] left_forward: {forward_l}, left_foot_height: {}, hip_pitch: {}", foot_height_l, og_left_leg_joints.hip_pitch);

        // let mut right_leg_joints = kinematics::inverse::right_leg_angles(FootPosition {
        //     forward: forward_r,
        //     left: left_r,
        //     turn: 0.0,
        //     hip_height: HIP_HEIGHT,
        //     foot_lift: foot_height_r,
        // });

        let (possible, mut left_leg_joints, mut right_leg_joints) = kinematics::hulk_ik::leg_angles(FootOffset {
            forward: forward_l,
            left: left_l,
            turn: 0.0,
            hip_height: HIP_HEIGHT,
            foot_lift: foot_height_l,
        }, FootOffset {
            forward: forward_r,
            left: left_r,
            turn: 0.0,
            hip_height: HIP_HEIGHT,
            foot_lift: foot_height_r,
        });

        info!("[HULK] left_forward: {forward_l}, left_foot_height: {}, left: {} hip_pitch: {}", foot_height_l, left_l, left_leg_joints.hip_pitch);
        info!("-------");
        // balance adjustment
        let balance_adjustment = walking_engine.filtered_gyro.y / 25.0;
        if next_foot_switch > 0.0 {
            match walking_engine.swing_side {
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

        control_message.position = JointArray::<f32>::builder()
            .left_shoulder_pitch(90f32.to_radians() + shoulder_pitch_l)
            .left_shoulder_roll(7f32.to_radians())
            .right_shoulder_pitch(90f32.to_radians() + shoulder_pitch_r)
            .right_shoulder_roll(-7f32.to_radians())
            .left_leg_joints(left_leg_joints)
            .right_leg_joints(right_leg_joints)
            .build();
    }

    Ok(())
}

fn has_support_foot_changed(side: &Side, fsr: &ForceSensitiveResistors) -> bool {
    let left_foot_pressure = fsr.left_foot.sum();
    let right_foot_pressure = fsr.right_foot.sum();
    (match side {
        Side::Left => left_foot_pressure,
        Side::Right => right_foot_pressure,
    }) > COP_PRESSURE_THRESHOLD
}

/// Filter the gyroscope values
fn filter_gyro_values(filtered_gyro: &Vector2<f32>, gyroscope: &Vector3<f32>) -> Vector2<f32> {
    Vector2 {
        x: 0.8 * filtered_gyro.x + 0.2 * gyroscope.x,
        y: 0.8 * filtered_gyro.y + 0.2 * gyroscope.y,
    }
}

fn left_angle(left: f32, hip_height: f32, linear_time: f32) -> f32 {
    let left_at_t = left * parabolic_step(linear_time);
    let height = hip_height - ANKLE_TO_SOLE.z.abs();

    (left_at_t / height).atan()
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

// fn parabolic_step(dt: f32, time: f32, period: f32, dead_time_fraction: f32) -> f32 {
//     let dead_time = period * dead_time_fraction / 2.0;
//     if time < dead_time + dt / 2.0 {
//         return 0.0;
//     }
//     if time > period - dead_time - dt / 2.0 {
//         return 1.0;
//     }
//     let time_fraction = (time - dead_time) / (period - 2.0 * dead_time);
//     if time < period / 2.0 {
//         return 2.0 * time_fraction * time_fraction;
//     }
//     return 4.0 * time_fraction - 2.0 * time_fraction * time_fraction - 1.0;
// }

fn linear_step(time: f32, period: f32) -> f32 {
    (time / period).clamp(0.0, 1.0)
}
