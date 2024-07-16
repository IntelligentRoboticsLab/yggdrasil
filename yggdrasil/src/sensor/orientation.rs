use nalgebra::{Quaternion, UnitComplex, UnitQuaternion, Vector3};
use nidhogg::types::ForceSensitiveResistors;
use serde::{Deserialize, Serialize};

use crate::{behavior::primary_state::PrimaryState, nao::CycleTime, prelude::*};

use super::imu::IMUValues;

const GRAVITY_CONSTANT: f32 = 9.81;

/// A module that uses the IMU data to maintain the current orientation of the robot.
///
/// This implementation is based on the paper <https://www.mdpi.com/1424-8220/15/8/19302/pdf>.
/// And implementation by the HULKs team.
///
/// The module provides the following resources to the application:
/// - [`RobotOrientation`]
pub struct OrientationFilter;

impl Module for OrientationFilter {
    fn initialize(self, app: App) -> Result<App> {
        app.add_staged_system(
            SystemStage::Sensor,
            update_orientation.after(super::imu::imu_sensor),
        )
        .add_startup_system(init_orientation_filter)
    }
}

#[startup_system]
pub fn init_orientation_filter(
    storage: &mut Storage,
    config: &OrientationFilterConfig,
) -> Result<()> {
    storage.add_resource(Resource::new(RobotOrientation::with_config(config)))?;
    Ok(())
}

#[system]
pub fn update_orientation(
    orientation: &mut RobotOrientation,
    imu: &IMUValues,
    fsr: &ForceSensitiveResistors,
    cycle: &CycleTime,
    primary_state: &PrimaryState,
) -> Result<()> {
    match primary_state {
        PrimaryState::Penalized | PrimaryState::Initial | PrimaryState::Unstiff => {
            orientation.reset();
        }
        _ => {
            orientation.update(imu, fsr, cycle);
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrientationFilterConfig {
    pub acceleration_weight: f32,
    pub acceleration_threshold: f32,
    pub gyro_threshold: f32,
    pub fsr_threshold: f32,
}

#[derive(Debug)]
pub struct RobotOrientation {
    pub orientation: UnitQuaternion<f32>,
    config: OrientationFilterConfig,
    gyro_t0: Vector3<f32>,
    gyro_bias: Vector3<f32>,
    initialized: bool,
}

impl RobotOrientation {
    /// Creates a new [`RobotOrientation`] with the provided configuration.
    pub fn with_config(config: &OrientationFilterConfig) -> Self {
        Self {
            orientation: UnitQuaternion::identity(),
            config: config.clone(),
            gyro_t0: Vector3::zeros(),
            gyro_bias: Vector3::zeros(),
            initialized: false,
        }
    }

    /// Updates the orientation of the robot based on the IMU data.
    pub fn update(&mut self, imu: &IMUValues, fsr: &ForceSensitiveResistors, cycle: &CycleTime) {
        let gyro = Vector3::new(imu.gyroscope.x, imu.gyroscope.y, imu.gyroscope.z);
        let linear_acceleration = Vector3::new(
            imu.accelerometer.x,
            imu.accelerometer.y,
            imu.accelerometer.z,
        );

        if !self.initialized {
            self.orientation = compute_initial(linear_acceleration);
            self.initialized = true;
            return;
        }

        if self.is_steady(
            gyro,
            linear_acceleration,
            fsr,
            self.config.gyro_threshold,
            self.config.acceleration_threshold,
            self.config.fsr_threshold,
        ) {
            // We cannot use a LowPassFilter here sadly, because it's implemented for nidhogg:Vector2,
            // and we want to use it for nalgebra::Vector3, making the type more complex.
            // https://github.com/IntelligentRoboticsLab/yggdrasil/issues/215
            self.gyro_bias = 0.01 * gyro + 0.99 * self.gyro_bias;
            self.gyro_t0 = gyro;
        } else {
            self.predict_next_orientation(gyro, cycle);
            self.apply_correction(linear_acceleration);
        }
        self.gyro_t0 = gyro;
    }

    /// Returns the current yaw of the robot, in 2D
    pub fn yaw(&self) -> UnitComplex<f32> {
        UnitComplex::new(self.orientation.inverse().euler_angles().2)
    }

    /// Predicts the next orientation based on the angular velocity and the cycle time.
    /// This uses equation 38 and 42 from the paper.
    fn predict_next_orientation(&mut self, gyro: Vector3<f32>, cycle: &CycleTime) {
        let orientation = self.orientation.quaternion();
        let gyro = gyro - self.gyro_bias;

        let rate = Quaternion::new(0.0, gyro.x, gyro.y, gyro.z);

        // equation 38
        let rate_derivative = -(rate * orientation) / 2.0;

        // equation 42
        self.orientation = UnitQuaternion::from_quaternion(
            orientation + rate_derivative * cycle.duration.as_secs_f32(),
        );
    }

    pub fn is_steady(
        &self,
        gyro: Vector3<f32>,
        linear_acceleration: Vector3<f32>,
        fsr: &ForceSensitiveResistors,
        gyro_threshold: f32,
        acceleration_threshold: f32,
        fsr_threshold: f32,
    ) -> bool {
        if (linear_acceleration.norm() - GRAVITY_CONSTANT).abs() > acceleration_threshold {
            return false;
        }

        let gyro_delta = (gyro - self.gyro_t0).abs();

        if gyro_delta.x > gyro_threshold
            || gyro_delta.y > gyro_threshold
            || gyro_delta.z > gyro_threshold
        {
            return false;
        }

        if fsr.left_foot.sum() < fsr_threshold || fsr.right_foot.sum() < fsr_threshold {
            return false;
        }

        true
    }

    /// Apply a correction to the orientation based on the linear acceleration and gravity.
    ///
    /// This is section 5.2.1 in the paper.
    fn apply_correction(&mut self, linear_acceleration: Vector3<f32>) {
        let orientation = self.orientation;
        let acceleration_weight = self.config.acceleration_weight;
        let linear_acceleration = linear_acceleration.normalize();

        // figure 5;
        // When the vehicle moves with high acceleration, the magnitude and direction of the total measured acceleration vector are different from gravity;
        // therefore the attitude is evaluated using a false reference, resulting in significant, possibly critical errors
        // To solve this we scale the weight of the acceleration correction based on the magnitude error
        let magnitude_error =
            (linear_acceleration.norm() - GRAVITY_CONSTANT).abs() / GRAVITY_CONSTANT;

        // threshold taken from paper (figure 5)
        let interpolation_factor = if magnitude_error <= 0.1 {
            acceleration_weight
        } else if magnitude_error <= 0.2 {
            10.0 * acceleration_weight * (0.2 - magnitude_error)
        } else {
            return;
        };

        // equation 44, use the predicted orientation to normalize the gravity vector into the global frame
        let projected_gravity = orientation.inverse().transform_vector(&linear_acceleration);

        // equation 47, compute the delta quaternion using the projected gravity vector
        let delta = UnitQuaternion::from_quaternion(Quaternion::new(
            ((projected_gravity.z + 1.0) / 2.0).sqrt(),
            -(projected_gravity.y / (2.0 * (projected_gravity.z + 1.0)).sqrt()),
            projected_gravity.x / (2.0 * (projected_gravity.z + 1.0)).sqrt(),
            0.0,
        ));

        // figure 4;
        // The delta may have a large value when the predicted gravity has a significant deviation from the real gravity.
        // If that condition does not occur, the delta quaternion is very small; thus, we prefer using the LERP formula because it is computationally more efficient.

        // equations 48, 49, 50, 51, 52
        // threshold taken from paper (0.9)
        let correction = if Quaternion::identity().dot(&delta) > 0.9 {
            UnitQuaternion::from_quaternion(
                UnitQuaternion::identity().lerp(&delta, interpolation_factor),
            )
        } else {
            UnitQuaternion::identity().slerp(&delta, interpolation_factor)
        };

        self.orientation *= correction;
    }

    fn reset(&mut self) {
        self.orientation = UnitQuaternion::identity();
        self.initialized = false;
    }
}

/// Computes the initial orientation of the robot based on the linear acceleration.
/// This is based on equation 25 in the paper.
fn compute_initial(linear_acceleration: Vector3<f32>) -> UnitQuaternion<f32> {
    let linear_acceleration = linear_acceleration.normalize();

    let (x, y, z, w) = if linear_acceleration.z >= 0.0 {
        (
            ((linear_acceleration.z + 1.0) / 2.0).sqrt(),
            -(linear_acceleration.y / (2.0 * (linear_acceleration.z + 1.0)).sqrt()),
            linear_acceleration.x / (2.0 * (linear_acceleration.z + 1.0)).sqrt(),
            0.0,
        )
    } else {
        (
            -(linear_acceleration.y / (2.0 * (1.0 - linear_acceleration.z)).sqrt()),
            ((1.0 - linear_acceleration.z) / 2.0).sqrt(),
            0.0,
            linear_acceleration.x / (2.0 * (1.0 - linear_acceleration.z)).sqrt(),
        )
    };

    UnitQuaternion::from_quaternion(Quaternion::new(x, y, z, w))
}
