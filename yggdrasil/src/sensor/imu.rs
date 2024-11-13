use crate::prelude::*;
use bevy::prelude::*;
use nalgebra::{Vector2, Vector3};
use nidhogg::NaoState;
use std::collections::VecDeque;
use std::ops::Div;

/// Amount of accelerometer measurements to calculate standard deviation over
const ACCELEROMETER_DEVIATION_WINDOW: usize = 50;

/// Plugin that offers a structured wrapper for the parts of the IMU,
/// derived from the raw [`NaoState`].
pub struct IMUSensorPlugin;

impl Plugin for IMUSensorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Sensor, imu_sensor)
            .init_resource::<IMUValues>();
    }
}

/// Run condition that returns `true` if a new IMU measurement has been received.
///
/// # Important
///
/// The IMU sensor in the NAO updates the gyroscope and accelerometer values
/// every other cycle respectively. This means that we only get a new gyroscope
/// measurement every 2 cycles, and a new accelerometer measurement every 2 other cycles.
#[must_use]
pub fn has_new_imu_sample(imu: Res<IMUValues>) -> bool {
    // we get a new gyroscope measurement every other cycle, and the
    // accelerometer measurement is updated in the cycles where the gyroscope
    // measurement is not updated, meaning we can check for a new sample by
    // checking if the gyroscope measurement has changed.
    imu.last_gyroscope != imu.gyroscope
}

/// Struct containing gyroscope, accelerometer and angles.
#[derive(Resource, Debug, Clone)]
pub struct IMUValues {
    /// The gyroscope provides direct measurements of the rotational speed along
    /// three axes (x, y and z) in radians per second (rad/s). The Z axis is facing up.
    ///
    /// Position relative to the torso frame: (-0.008, 0.006, 0.029) in m.
    pub gyroscope: Vector3<f32>,
    /// The gyroscope measurement from the last cycle, in radians per second.
    ///
    /// Used to check whether a new IMU sample has been received.
    last_gyroscope: Vector3<f32>,
    /// The accelerometer measures the proper acceleration along three axes (x, y, and z)
    /// in meters per second squared (m/s²). The Z axis is facing up.
    ///
    /// Position relative to the torso frame: (-0.008, 0.00606, 0.027) in m.
    pub accelerometer: Vector3<f32>,
    /// The accelerometer measurement from the last cycle, in meters per second squared.
    ///
    /// Used to check whether a new IMU sample has been received.
    last_accelerometer: Vector3<f32>,
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
            last_gyroscope: Vector3::default(),
            accelerometer: Vector3::default(),
            last_accelerometer: Vector3::default(),
            angles: Vector2::default(),
            accelerometer_variance: Vector3::default(),
            accelerometer_measurements,
        }
    }
}

impl IMUValues {
    /// Returns `true` if a new gyroscope measurement has been received.
    #[must_use]
    pub fn has_new_gyroscope_measurement(&self) -> bool {
        self.last_gyroscope != self.gyroscope
    }

    /// Returns `true` if a new accelerometer measurement has been received.
    #[must_use]
    pub fn has_new_accelerometer_measurement(&self) -> bool {
        self.last_accelerometer != self.accelerometer
    }
}

/// Calculate the variance of `measurements`, multiplied by `ACCELEROMETER_DEVIATION_WINDOW`.
fn variance(measurements: &VecDeque<Vector3<f32>>) -> Vector3<f32> {
    let measurement_avg: Vector3<f32> = measurements
        .iter()
        .sum::<Vector3<f32>>()
        .div(measurements.len() as f32);

    measurements.iter().fold(Vector3::default(), |acc, item| {
        let diff: Vector3<f32> = measurement_avg - item;
        acc + diff.component_mul(&diff)
    })
}

pub(super) fn imu_sensor(nao_state: Res<NaoState>, mut imu_values: ResMut<IMUValues>) {
    // The IMU sensor in the NAO updates the gyroscope and accelerometer values
    // every other cycle respectively.
    // This means that we only get a new gyroscope value every 2 cycles, and a new
    // accelerometer value every 2 other cycles.
    //
    // We store the last gyroscope and accelerometer values to be able to check whether
    // a new sample has been received in the current cycle.
    imu_values.last_gyroscope = imu_values.gyroscope;
    imu_values.last_accelerometer = imu_values.accelerometer;

    imu_values.gyroscope = nao_state.gyroscope;
    imu_values.accelerometer = nao_state.accelerometer;
    imu_values.angles = nao_state.angles;

    imu_values.accelerometer_measurements.pop_front();
    imu_values
        .accelerometer_measurements
        .push_back(nao_state.accelerometer);

    imu_values.accelerometer_variance = variance(&imu_values.accelerometer_measurements);
}
