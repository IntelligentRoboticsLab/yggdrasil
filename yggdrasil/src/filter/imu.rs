use std::collections::VecDeque;
use std::iter::Sum;

use crate::prelude::*;
use nidhogg::{types::Vector2, types::Vector3, NaoState};

/// A module offering a structured wrapper for the parts of the IMU, derived from the raw [`NaoState`].
///
/// This module provides the following resources to the application:
/// - [`IMUValues`]
pub struct IMUFilter;

impl Module for IMUFilter {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(imu_filter).init_resource::<IMUValues>()
    }
}

/// Struct containing gyroscope, accelerometer and angles.
#[derive(Clone)]
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

    pub accelerometer_std: Vector3<f32>,

    accelerometer_measurements: VecDeque<Vector3<f32>>,
}

impl Default for IMUValues {
    fn default() -> Self {
        IMUValues {
            gyroscope: Vector3::default(),
            accelerometer: Vector3::default(),
            angles: Vector2::default(),
            accelerometer_std: Vector3::default(),
            accelerometer_measurements: VecDeque::with_capacity(50),
        }
    }
}

// impl Sum<Vector3<f32>> for Vector3<f32> {
//     fn sum<I: Iterator<Item = Vector3<f32>>>(iter: I) -> Self {
//         // iter.y
//         Vector3::default()
//     }
// }

fn standard_deviation(measurements: &VecDeque<Vector3<f32>>) -> Vector3<f32> {
    // let avg: Vector3<f32> = measurements.iter().sum();
    //  / measurements.len() as f32;

    // let variance: f32 = array
    //     .iter()
    //     .map(|val| (val - avg) * (val - avg))
    //     .sum::<f32>()
    //     / array.len() as f32;

    // variance
    // avg
    Vector3::default()
}

#[system]
pub fn imu_filter(nao_state: &NaoState, imu_values: &mut IMUValues) -> Result<()> {
    imu_values.gyroscope = nao_state.gyroscope;
    imu_values.accelerometer = nao_state.accelerometer;
    imu_values.angles = nao_state.angles;

    imu_values
        .accelerometer_measurements
        .push_back(nao_state.accelerometer.clone());

    if imu_values.accelerometer_measurements.len() > 50 {
        imu_values.accelerometer_measurements.pop_front();
    }

    imu_values.accelerometer_std = standard_deviation(&imu_values.accelerometer_measurements);

    Ok(())
}
