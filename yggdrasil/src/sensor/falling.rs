use crate::prelude::*;
use crate::sensor::imu::IMUValues;

/// Minimum angle for falling detection.
const MIN_FALL_ANGLE_FORWARDS: f32 = 0.45;
const MIN_FALL_ANGLE_BACKWARDS: f32 = -0.45;
const MIN_FALL_ANGLE_LEFT: f32 = -0.52;
const MIN_FALL_ANGLE_RIGHT: f32 = 0.52;
// Minimum angle for lying confirmation.
const MIN_LYING_ANGLE: f32 = 1.3;
/// Minimum accelerometer deviation for lying confirmation.
const MAX_ACC_DEVIATION: f32 = 0.175;

/// A module offering a Pose resource, containing the current pose state of the robot, and rudimentary falling detection.
///
/// This module provides the following resources to the application:
/// - [`FallState`]
pub struct FallingFilter;

impl Module for FallingFilter {
    fn initialize(self, app: App) -> Result<App> {
        app.add_staged_system(SystemStage::Sensor, pose_filter)
            .init_resource::<FallState>()
    }
}

/// FallState contains the variants: Falling, Upright and Lying. Both Falling and Lying have their
/// associated values which are again, enum types containing the directions the robot can fall or
/// lie in.

#[derive(Default, Clone, Debug)]
pub enum FallState {
    Falling(FallDirection),
    #[default]
    None,
    Lying(LyingDirection),
}

/// FallDirection contains four variants which are associated with the direction of the fall.
#[derive(Clone, Debug)]
pub enum FallDirection {
    Forwards,
    Backwards,
    Left,
    Right,
}

/// LyingDirection contains two variants which are associated with the position of a fallen robot.
#[derive(Clone, Debug)]
pub enum LyingDirection {
    FacingUp,
    FacingDown,
}

/// Is the robot falling forward based on its angle and gyroscope.
fn is_falling_forward(imu_values: &IMUValues) -> bool {
    imu_values.angles.y > MIN_FALL_ANGLE_FORWARDS
}

/// Is the robot falling backwards based on its angle and gyroscope.
fn is_falling_backward(imu_values: &IMUValues) -> bool {
    imu_values.angles.y < MIN_FALL_ANGLE_BACKWARDS
}

/// Is the robot falling left based on its angle and gyroscope.
fn is_falling_left(imu_values: &IMUValues) -> bool {
    imu_values.angles.x < MIN_FALL_ANGLE_LEFT
}

/// Is the robot falling right based on its angle and gyroscope.
fn is_falling_right(imu_values: &IMUValues) -> bool {
    imu_values.angles.x > MIN_FALL_ANGLE_RIGHT
}

/// Is the robot lying on its stomach based on the accelerometer and angle.
fn is_lying_on_stomach(imu_values: &IMUValues) -> bool {
    imu_values.accelerometer_variance.y < MAX_ACC_DEVIATION
        && imu_values.angles.y >= MIN_LYING_ANGLE
}

/// Is the robot lying on its back based on the accelerometer and angle.
fn is_lying_on_back(imu_values: &IMUValues) -> bool {
    imu_values.accelerometer_variance.y < MAX_ACC_DEVIATION
        && imu_values.angles.y <= -MIN_LYING_ANGLE
}

/// Checks position of the robot and sets [`FallState`], [`FallDirection`] and [`LyingDirection`]
/// accordingly.
#[system]
fn pose_filter(imu_values: &IMUValues, fall_state: &mut FallState) -> Result<()> {
    let is_lying_on_stomach = is_lying_on_stomach(imu_values);
    let is_lying_on_back = is_lying_on_back(imu_values);

    if is_falling_forward(imu_values) && !is_lying_on_stomach {
        *fall_state = FallState::Falling(FallDirection::Forwards);
    } else if is_falling_backward(imu_values) && !is_lying_on_back {
        *fall_state = FallState::Falling(FallDirection::Backwards);
    } else if is_falling_left(imu_values) && !is_lying_on_stomach && !is_lying_on_back {
        *fall_state = FallState::Falling(FallDirection::Left);
    } else if is_falling_right(imu_values) && !is_lying_on_stomach && !is_lying_on_back {
        *fall_state = FallState::Falling(FallDirection::Right);
    } else if is_lying_on_stomach {
        *fall_state = FallState::Lying(LyingDirection::FacingDown);
    } else if is_lying_on_back {
        *fall_state = FallState::Lying(LyingDirection::FacingUp);
    } else {
        *fall_state = FallState::None;
    }

    Ok(())
}
