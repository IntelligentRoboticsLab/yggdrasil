use crate::prelude::*;
use nidhogg::{types::Vector2, types::Vector3, NaoState};

/// A module offering a structured wrapper for the parts of the IMU, derived from the raw [`NaoState`].
///
/// This module provides the following resources to the application:
/// - [`IMUValues`]
///
pub struct IMUFilter;

impl Module for IMUFilter {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(imu_filter).init_resource::<IMUValues>()
    }
}

/// Struct containing gyroscope, accelerometer and angles.
#[derive(Default)]
pub struct IMUValues {
    /// The Gyroscope provides direct measurements of the rotational speed along
    /// three axes (x, y and z) in radians per second (rad/s). The Z axis is facing up.
    ///
    /// Position relative to the torso frame: (-0.008, 0.006, 0.029) in m.
    pub gyroscope: Vector3<f32>,
    /// The Accelerometer measures the proper acceleration along three axes (x, y, and z)
    /// in meters per second squared (m/s²). The Z axis is facing up.
    ///
    /// Position relative to the torso frame: (-0.008, 0.00606, 0.027) in m.
    pub accelerometer: Vector3<f32>,
    /// Two inclination angles (x, y) of the robot's body.
    ///
    /// These angles represent the orientation of the robot and are measured in radians.
    pub angles: Vector2<f32>,
}

#[system]
fn imu_filter(nao_state: &NaoState, imu_values: &mut IMUValues) -> Result<()> {
    imu_values.gyroscope = nao_state.gyroscope.clone();
    imu_values.accelerometer = nao_state.accelerometer.clone();
    imu_values.angles = nao_state.angles.clone();

    Ok(())
}
