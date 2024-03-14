use crate::filter::{fsr::Contacts, imu::IMUValues};
use miette::Result;
use tyr::prelude::*;

/// Maximum angle for standing upright.
const MAX_UPRIGHT_ANGLE: f32 = 0.1;
/// Minimum angle for falling detection.
const MIN_FALL_ANGLE_FORWARDS: f32 = 0.45;
const MIN_FALL_ANGLE_BACKWARDS: f32 = -0.45;
const MIN_FALL_ANGLE_LEFT: f32 = -0.52;
const MIN_FALL_ANGLE_RIGHT: f32 = 0.52;
/// Minimum velocity for falling detection.
const MIN_FALL_VELOCITY_FORWARDS: f32 = 0.15;
const MIN_FALL_VELOCITY_BACKWARDS: f32 = 0.15;
const MIN_FALL_VELOCITY_LEFT: f32 = 0.15;
const MIN_FALL_VELOCITY_RIGHT: f32 = 0.15;
// Minimum angle for lying confirmation.
const MIN_LYING_ANGLE: f32 = 1.5;
/// Minimum accelerometer deviation for lying confirmation.
const MAX_ACC_DEVIATION: f32 = 0.175;

/// A module offering a Pose resource, containing the current pose state of the robot, and rudimentary falling detection.
///
/// This module provides the following resources to the application:
/// - [`Fall`]
pub struct FallingFilter;

impl Module for FallingFilter {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(pose_filter)
            .init_resource::<Fall>()
    }
}

/// Struct containing the current FallState of the NAO.
#[derive(Default)]
pub struct Fall {
    pub state: FallState,
}

/// FallState contains the variants: Falling, Upright and Lying. Both Falling and Lying have their
/// associated values which are again, enum types containing the directions the robot can fall or
/// lie in.
#[derive(Default, Clone, Debug)]
pub enum FallState {
    Falling(FallDirection),
    #[default]
    Upright,
    Lying(LyingDirection),
}

/// FallDirection contains four variants which are associated with the direction of the fall.
#[derive(Clone, Debug)]
pub enum FallDirection {
    Forwards,
    Backwards,
    Leftways,
    Rightways,
}

/// LyingDirection contains two variants which are associated with the position of a fallen robot.
#[derive(Clone, Debug)]
pub enum LyingDirection {
    FacingUp,
    FacingDown,
}

/// Is the robot falling forward based on its angle and gyroscope.
fn is_falling_forward(imu_values: &IMUValues) -> bool {
    (imu_values.angles.y > MIN_FALL_ANGLE_FORWARDS) && imu_values.gyroscope.y.abs() > MIN_FALL_VELOCITY_FORWARDS
}
/// Is the robot falling backwards based on its angle and gyroscope.
fn is_falling_backward(imu_values: &IMUValues) -> bool {
    (imu_values.angles.y < MIN_FALL_ANGLE_BACKWARDS) && imu_values.gyroscope.y.abs() > MIN_FALL_VELOCITY_BACKWARDS
}
/// Is the robot falling left based on its angle and gyroscope.
fn is_falling_left(imu_values: &IMUValues) -> bool {
    (imu_values.angles.x < MIN_FALL_ANGLE_LEFT) && imu_values.gyroscope.x.abs() > MIN_FALL_VELOCITY_LEFT
}
/// Is the robot falling right based on its angle and gyroscope.
fn is_falling_right(imu_values: &IMUValues) -> bool {
    (imu_values.angles.x > MIN_FALL_ANGLE_RIGHT) && imu_values.gyroscope.x.abs() > MIN_FALL_VELOCITY_RIGHT
}

/// Is the robot standing upright based on its angles and ground contact.
fn is_standing_upright(imu_values: &IMUValues, contacts: &Contacts) -> bool {
    imu_values.angles.x < MAX_UPRIGHT_ANGLE
        && imu_values.angles.y < MAX_UPRIGHT_ANGLE
        && contacts.ground
}

/// Is the robot lying on its stomach based on the accelerometer and angle.
fn is_lying_on_stomach(imu_values: &IMUValues) -> bool {
    imu_values.accelerometer_std.y < MAX_ACC_DEVIATION && imu_values.angles.y >= MIN_LYING_ANGLE
}
/// Is the robot lying on its back based on the accelerometer and angle.
fn is_lying_on_back(imu_values: &IMUValues) -> bool {
    imu_values.accelerometer_std.y < MAX_ACC_DEVIATION && imu_values.angles.y <= -MIN_LYING_ANGLE
}

/// Checks position of the robot and sets [`FallState`], [`FallDirection`] and [`LyingDirection`]
/// accordingly.
#[system]
fn pose_filter(imu_values: &IMUValues, fallingstate: &mut Fall, contacts: &Contacts) -> Result<()> {
    if is_falling_forward(imu_values) {
        fallingstate.state = FallState::Falling(FallDirection::Forwards);
        println!("falling forward");
    } else if is_falling_backward(imu_values) {
        fallingstate.state = FallState::Falling(FallDirection::Backwards);
        println!("falling backward");
    } else if is_falling_left(imu_values) {
        fallingstate.state = FallState::Falling(FallDirection::Leftways);
        println!("falling left");
    } else if is_falling_right(imu_values) {
        fallingstate.state = FallState::Falling(FallDirection::Rightways);
        println!("falling right");
    }

    if is_standing_upright(imu_values, contacts) {
        fallingstate.state = FallState::Upright;
    }

    if is_lying_on_stomach(imu_values) {
        fallingstate.state = FallState::Lying(LyingDirection::FacingDown);
    } else if is_lying_on_back(imu_values) {
        fallingstate.state = FallState::Lying(LyingDirection::FacingUp);
    }

    Ok(())
}
