use crate::prelude::*;
use miette::Result;
use nidhogg::types::FillExt;
use nidhogg::{types::Vector2, types::Vector3, NaoState};
use num::traits::pow::Pow;
use std::collections::VecDeque;
use std::ops::Div;

/// Amount of accelerometer measurements to calculate standard deviation over
const ACCELEROMETER_DEVIATION_WINDOW: usize = 50;

/// A module offering a structured wrapper for the parts of the IMU, derived from the raw [`NaoState`].
///
/// This module provides the following resources to the application:
/// - [`IMUValues`]
pub struct IMUSensor;

impl Module for IMUSensor {
    fn initialize(self, app: App) -> Result<App> {
        app.add_staged_system(SystemStage::Sensor, imu_sensor)
            .init_resource::<IMUValues>()
    }
}

/// Struct containing gyroscope, accelerometer and angles.
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

    /// The variance of the accelerometer measurements, over a window of `ACCELEROMETER_DEVIATION_WINDOW` samples.
    pub accelerometer_variance: Vector3<f32>,

    /// The last `ACCELEROMETER_DEVIATION_WINDOW` accelerometer measurements.
    accelerometer_measurements: VecDeque<Vector3<f32>>,
}

impl Default for IMUValues {
    fn default() -> Self {
        let mut accelerometer_measurements =
            VecDeque::with_capacity(ACCELEROMETER_DEVIATION_WINDOW);
        for _ in 0..ACCELEROMETER_DEVIATION_WINDOW {
            accelerometer_measurements.push_back(Vector3::default());
        }

        IMUValues {
            gyroscope: Vector3::default(),
            accelerometer: Vector3::default(),
            angles: Vector2::default(),
            accelerometer_variance: Vector3::default(),
            accelerometer_measurements,
        }
    }
}

/// Calculate the variance of `measurements`, multiplied by `ACCELEROMETER_DEVIATION_WINDOW`.
fn variance(measurements: &VecDeque<Vector3<f32>>) -> Vector3<f32> {
    let measurement_avg: Vector3<f32> = measurements
        .iter()
        .sum::<Vector3<f32>>()
        .div(Vector3::fill(measurements.len() as f32));

    measurements.iter().fold(Vector3::default(), |acc, item| {
        let diff: Vector3<f32> = measurement_avg - item;
        acc + diff.pow(2)
    })
}

#[system]
pub fn imu_sensor(nao_state: &NaoState, imu_values: &mut IMUValues) -> Result<()> {
    imu_values.gyroscope = nao_state.gyroscope;
    imu_values.accelerometer = nao_state.accelerometer;
    imu_values.angles = nao_state.angles;

    imu_values.accelerometer_measurements.pop_front();
    imu_values
        .accelerometer_measurements
        .push_back(nao_state.accelerometer);

    imu_values.accelerometer_variance = variance(&imu_values.accelerometer_measurements);

    Ok(())
}
