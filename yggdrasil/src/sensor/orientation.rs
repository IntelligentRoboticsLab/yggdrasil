use super::imu::IMUValues;
use crate::{behavior::primary_state::PrimaryState, localization::odometry::Odometry, prelude::*};
use bevy::prelude::*;
use nalgebra::{Quaternion, UnitQuaternion, Vector3};
use serde::{Deserialize, Serialize};
use serde_with::{DurationMilliSeconds, DurationSeconds, serde_as};
use std::time::Duration;
use vqf::Vqf;

/// The NAO's IMU update rate.
///
/// This is actually 41Hz and not 82Hz (Richter-Klug, 2018)
const IMU_RATE: f32 = 41.0;

/// Plugin which maintains the robot's orientation using the IMU data.
///
/// This implementation is based on the VQF described in [this paper](https://arxiv.org/pdf/2203.17024).
pub struct OrientationFilterPlugin;

impl Plugin for OrientationFilterPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Sensor,
            update_orientation
                .after(super::imu::imu_sensor)
                .run_if(super::imu::has_new_imu_sample),
        )
        .add_systems(Startup, init_vqf)
        .add_systems(PreUpdate, reset_orientation);
    }
}

/// Orientation of the robot in 3D space, based on a VQF filter.
#[derive(Resource, Deref, DerefMut)]
pub struct RobotOrientation {
    /// The inner VQF filter.
    ///
    /// See [`Vqf`] for more information.
    #[deref]
    vqf: Vqf,
    /// Offset of the yaw angle in radians.
    ///
    /// The VQF algorithm cannot determine the yaw angle without a magnetometer,
    /// it will always be relative to some initial orientation, which can be computed
    /// from the accelerometer data. This offset is then stored here and added to
    /// the yaw angle to get the absolute orientation.
    yaw_offset: Option<UnitQuaternion<f32>>,
}

impl RobotOrientation {
    /// Returns whether the orientation filter is initialized.
    #[must_use]
    pub fn is_initialized(&self) -> bool {
        self.yaw_offset.is_some()
    }

    /// Initializes the orientation filter.
    fn initialize(&mut self) {
        let (_, _, yaw) = self.vqf.orientation().euler_angles();
        // set the offset to the current yaw angle
        self.yaw_offset = Some(UnitQuaternion::from_euler_angles(0., 0., -yaw));
    }

    /// Resets the accumulated orientation, and the filter values.
    #[allow(unused)]
    pub fn reset(&mut self) {
        self.yaw_offset = None;
        self.vqf.reset_orientation(UnitQuaternion::identity());
    }

    /// Returns the current orientation of the robot.
    #[inline]
    #[must_use]
    pub fn quaternion(&self) -> UnitQuaternion<f32> {
        let imu_to_robot_frame =
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), std::f32::consts::PI)
                * UnitQuaternion::from_quaternion(Quaternion::new(
                    0.,
                    1. / 2_f32.sqrt(),
                    1. / 2_f32.sqrt(),
                    0.,
                ));

        if let Some(offset) = self.yaw_offset {
            imu_to_robot_frame * (offset * self.vqf.orientation())
        } else {
            imu_to_robot_frame * self.vqf.orientation()
        }
    }

    #[inline]
    #[must_use]
    pub fn euler_angles(&self) -> (f32, f32, f32) {
        self.quaternion().euler_angles()
    }

    #[inline]
    #[must_use]
    pub fn is_resting(&self) -> bool {
        self.vqf.is_rest_phase()
    }
}

fn init_vqf(mut commands: Commands, config: Res<OrientationFilterConfig>) {
    let imu_sample_period = Duration::from_secs_f32(1.0 / IMU_RATE);

    let vqf = Vqf::new(imu_sample_period, imu_sample_period, config.as_ref().into());
    commands.insert_resource(RobotOrientation {
        vqf,
        yaw_offset: None,
    });
}

/// System that resets the orientation each cycle, iff we're in a state that doesn't need orientation data.
fn reset_orientation(
    mut orientation: ResMut<RobotOrientation>,
    primary_state: Res<PrimaryState>,
    mut prev_state: Local<Option<PrimaryState>>,
) {
    if prev_state.is_none() {
        *prev_state = Some(*primary_state);
        return;
    }

    if let Some(prev_state) = *prev_state {
        if (prev_state != PrimaryState::Standby && *primary_state == PrimaryState::Standby)
            || (prev_state == PrimaryState::Penalized && *primary_state != PrimaryState::Penalized)
        {
            orientation.reset();
        }
    }

    *prev_state = Some(*primary_state);
}

pub fn update_orientation(
    mut orientation: ResMut<RobotOrientation>,
    mut odometry: ResMut<Odometry>,
    imu: Res<IMUValues>,
) {
    orientation.update(imu.gyroscope, imu.accelerometer);

    if !orientation.is_initialized() {
        orientation.initialize();
        odometry.reset_orientation(&orientation);
    }
}

/// Configuration for the orientation filter.
///
/// this is an exact copy of [`vqf::VqfParameters`], but with [`serde_with`]
/// attributes added to make it nice to serialize and deserialize.
#[serde_as]
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct OrientationFilterConfig {
    /// Time constant τ<sub>acc</sub> for accelerometer low-pass filtering.
    ///
    /// Small values for τ<sub>acc</sub> imply trust on the accelerometer
    /// measurements, while large values of τ<sub>acc</sub> imply trust on the
    /// gyroscope measurements.
    ///
    /// The time constant τ<sub>acc</sub> corresponds to the cutoff frequency ƒ<sub>c</sub>
    /// of the second-order Butterworth low-pass filter as follows: f<sub>c</sub> = √2 / (2 π τ<sub>acc</sub>)
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub tau_accelerometer: Duration,
    /// Enables gyroscope bias estimation during motion phases.
    ///
    /// # Note
    ///
    /// Gyroscope bias is estimated based on the inclination correction only!
    pub do_bias_estimation: bool,
    /// Enables gyroscope bias estimation during rest phases.
    ///
    /// # Note
    ///
    /// This enables "rest"-phase detection, phases in which the IMU is at rest.
    /// During rest-phases, the gyroscope bias is estimated from the
    /// low-pass filtered gyroscope readings.
    pub do_rest_bias_estimation: bool,
    /// Standard deviation of the initial bias estimation uncertainty, in
    /// degrees per second.
    pub bias_sigma_initial: f32,
    /// Time in which the bias estimation uncertainty increases from 0 °/s to
    /// 0.1 °/s. This value determines the system noise assumed by the
    /// Kalman filter.
    #[serde_as(as = "DurationSeconds<u64>")]
    pub bias_forgetting_time: Duration,
    /// Maximum expected gyroscope bias, in degrees per second.
    ///
    /// This value is used to clip the bias estimate and the measurement error
    /// in the bias estimation update step.
    /// It is further used by the rest detection algorithm in order to not
    /// regard measurements with a large but constant angular rate as rest.
    pub bias_clip: f32,
    /// Standard deviation of the converged bias estimation uncertainty during
    /// motion, in degrees per second.
    pub bias_sigma_motion: f32,
    /// Forgetting factor for unobservable bias in vertical direction during
    /// motion.
    ///
    /// As magnetometer measurements are deliberately not used during motion
    /// bias estimation, gyroscope bias is not observable in vertical
    /// direction.
    ///
    /// This value is the relative weight of an artificial zero measurement that
    /// ensures that the bias estimate in the unobservable direction will
    /// eventually decay to zero.
    pub bias_vertical_forgetting_factor: f32,
    /// Standard deviation of the converged bias estimation uncertainty during a
    /// rest phase, in degrees per second.
    pub bias_sigma_rest: f32,
    /// Time threshold for rest detection.
    ///
    /// A rest phase is detected when the measurements have been close to the
    /// low-pass filtered reference for at least this duration.
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub rest_min_duration: Duration,
    /// Time constant for the low-pass filter used in the rest detection.
    ///
    /// This time constant characterizes a second-order Butterworth low-pass
    /// filter used to obtain the reference for rest detection.
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub rest_filter_tau: Duration,
    /// Angular velocity threshold for rest detection, in degrees per second.
    ///
    /// For a rest phase to be detected, the norm of the deviation between
    /// measurement and reference must be below the provided threshold.
    /// The absolute value of each component must also be below
    /// [`Self::bias_clip`].
    pub rest_threshold_gyro: f32,
    /// Acceleration threshold for rest phase detection in m/s<sup>2</sup>.
    ///
    /// For a rest phase to be detected, the norm of the deviation between
    /// measurement and reference must be below the provided threshold.
    pub rest_threshold_accel: f32,
}

impl From<&OrientationFilterConfig> for vqf::VqfParameters {
    fn from(config: &OrientationFilterConfig) -> Self {
        Self {
            tau_accelerometer: config.tau_accelerometer,
            do_bias_estimation: config.do_bias_estimation,
            do_rest_bias_estimation: config.do_rest_bias_estimation,
            bias_sigma_initial: config.bias_sigma_initial,
            bias_forgetting_time: config.bias_forgetting_time,
            bias_clip: config.bias_clip,
            bias_sigma_motion: config.bias_sigma_motion,
            bias_vertical_forgetting_factor: config.bias_vertical_forgetting_factor,
            bias_sigma_rest: config.bias_sigma_rest,
            rest_min_duration: config.rest_min_duration,
            rest_filter_tau: config.rest_filter_tau,
            rest_threshold_gyro: config.rest_threshold_gyro,
            rest_threshold_accel: config.rest_threshold_accel,
        }
    }
}
