use std::time::{Duration, Instant};

use super::SensorConfig;
use crate::prelude::*;
use crate::sensor::low_pass_filter::LowPassFilter;
use bevy::prelude::*;
use nalgebra::SVector;

use crate::motion::walk::SwingFootSwitchedEvent;
use nidhogg::{
    types::{FillExt, Fsr, FsrFoot},
    NaoState,
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};

// Omega for the low-pass filter.
const OMEGA: f32 = 0.7;

/// Plugin offering the Force Sensitive Resistor (FSR) sensor data of the Nao,
/// derived from the raw [`NaoState`].
pub struct FSRSensorPlugin;

impl Plugin for FSRSensorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Fsr>();
        app.init_resource::<Contacts>();

        app.add_systems(PostStartup, init_fsr_calibration);
        app.add_systems(
            Sensor,
            (
                force_sensitive_resistor_sensor,
                update_contacts,
                update_fsr_calibration,
            )
                .chain(),
        );
        app.add_systems(
            Update,
            update_min_pressure.run_if(on_event::<SwingFootSwitchedEvent>),
        );
    }
}

/// Configuration for FSR sensor data.
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FsrConfig {
    /// Threshold for ground contact detection using average FSR sensor values from both feet.
    pub ground_contact_threshold: f32,

    /// Timeout for change of value of the ground contact state in milliseconds.
    #[serde_as(as = "DurationMilliSeconds")]
    pub ground_contact_timeout: Duration,

    /// Maximum amount of pressure measured by a single sensor.
    pub max_pressure: f32,

    /// Initial value for minimum pressure measured by a single sensor.
    pub min_pressure: f32,

    /// The time to sample pressure values before updating the maximum value for each sensor, in milliseconds.
    #[serde_as(as = "DurationMilliSeconds")]
    pub highest_pressure_update_rate: Duration,

    /// The number of foot switches required before updating the minimum value for each sensor.
    pub num_foot_switches: u32,
}

impl FsrConfig {
    /// Get a [`FsrFoot`] with each value set to [`Self::min_pressure`].
    fn min_pressure_foot(&self) -> FsrFoot {
        FsrFoot::fill(self.min_pressure)
    }

    /// Get a [`FsrFoot`] with each value set to [`Self::max_pressure`].
    fn max_pressure_foot(&self) -> FsrFoot {
        FsrFoot::fill(self.max_pressure)
    }
}

/// Struct containing the various contact points of the Nao.
#[derive(Resource)]
pub struct Contacts {
    /// Whether the robot has ground contact.
    pub ground: bool,

    /// Whether the robot's left foot has ground contact.
    pub left_foot: bool,

    /// Whether the robot's right foot has ground contact.
    pub right_foot: bool,

    /// Timestamp of detected change in pressure state.
    pub last_switched: Instant,

    /// The first-order Butterworth low-pass filter to apply to the FSR sensor data.
    pub lpf: LowPassFilter<1>,
}

impl Default for Contacts {
    fn default() -> Self {
        Contacts {
            ground: true,
            left_foot: false,
            right_foot: false,
            last_switched: Instant::now(),
            lpf: LowPassFilter::new(OMEGA),
        }
    }
}

pub fn force_sensitive_resistor_sensor(nao_state: Res<NaoState>, mut fsr: ResMut<Fsr>) {
    fsr.left_foot = nao_state.fsr.left_foot.clone();
    fsr.right_foot = nao_state.fsr.right_foot.clone();
}

fn update_contacts(
    config: Res<SensorConfig>,
    fsr: Res<Fsr>,
    mut contacts: ResMut<Contacts>,
    mut last_pressure: Local<bool>,
) {
    let config = &config.fsr;

    contacts.left_foot = fsr.left_foot.sum() >= config.min_pressure;
    contacts.right_foot = fsr.right_foot.sum() >= config.min_pressure;

    // Retrieve FSR values and apply low-pass filter.
    let fsr_vector = SVector::<f32, 1>::from([fsr.avg()]);
    let filtered_fsr = contacts.lpf.update(fsr_vector);
    let current_pressure = filtered_fsr.x > config.ground_contact_threshold;

    if current_pressure != *last_pressure {
        contacts.last_switched = Instant::now();
    }

    // Only update the ground state if timeout duration has elapsed.
    if contacts.last_switched.elapsed() >= (config.ground_contact_timeout) {
        contacts.ground = current_pressure;
    }

    *last_pressure = current_pressure;
}

#[derive(Debug, Clone, Default)]
pub struct FsrFootCalibrationState {
    max: FsrFoot,
    new_max: FsrFoot,
    min: FsrFoot,
    new_min: FsrFoot,
}

impl FsrFootCalibrationState {
    fn init(config: &FsrConfig) -> Self {
        Self {
            max: config.min_pressure_foot(),
            new_max: config.min_pressure_foot(),
            min: config.max_pressure_foot(),
            new_min: config.max_pressure_foot(),
        }
    }

    /// Update the running max and min pressure values.
    ///
    /// If for whatever reason we come across a value that's > the current max, we update
    /// the current max as well.
    fn update(&mut self, reading: &FsrFoot) {
        self.max = self.max.max_per_sensor(reading);
        self.new_max = self.new_max.max_per_sensor(reading);
        self.new_min = self.new_min.min_per_sensor(reading);
    }

    /// Update the current maximum pressure value, based on the maximum readings collected
    /// over the last update duration.
    fn recalibrate_max_pressure(&mut self, min_pressure: FsrFoot) {
        self.max = self.new_max.clone();
        self.new_max = min_pressure;
    }

    /// Update the current minimum pressure value, based on the minimum readings collected
    /// over the last N foot switches.
    fn recalibrate_min_pressure(&mut self, max_pressure: FsrFoot) {
        self.min = std::mem::replace(&mut self.new_min, max_pressure);
    }
}

/// Resource that maintains calibrated values for the FSR sensors.
///
/// This struct contains normalized FSR values, based on the maximum and minimum
/// pressure values over the last N cycles. These values range from 0-1 and can be accessed through
/// [`CalibratedFsr::normalized`].
///
/// # Important
///
/// Calibrated values should only be trusted if [`CalibratedFsr::is_calibrated`] equals `true`!
#[derive(Resource, Debug, Clone)]
pub struct CalibratedFsr {
    /// Whether the maximum and minimum pressure values have been calibrated.
    pub is_calibrated: bool,
    /// The normalized FSR readings for this cycle.
    pub normalized: Fsr,
    /// Calibrated FSR state for the left foot.
    left: FsrFootCalibrationState,
    /// Calibrated FSR state for the right foot.
    right: FsrFootCalibrationState,
    /// The maximum pressure ever recorded on a single foot.
    max_pressure: f32,
    /// The timestamp when the calibration was last updated.
    last_updated: Instant,
}

impl CalibratedFsr {
    #[must_use]
    pub fn init(config: &FsrConfig) -> Self {
        Self {
            left: FsrFootCalibrationState::init(config),
            right: FsrFootCalibrationState::init(config),
            max_pressure: config.min_pressure_foot().sum(),
            last_updated: Instant::now(),
            is_calibrated: false,
            normalized: Fsr::default(),
        }
    }

    fn update_foot_pressure(&mut self, fsr: &Fsr) {
        self.normalized = fsr.clone()
            / Fsr {
                left_foot: self.left.max.clone(),
                right_foot: self.right.max.clone(),
            };
    }

    #[must_use]
    pub fn min_pressure_left(&self) -> &FsrFoot {
        &self.left.min
    }

    #[must_use]
    pub fn min_pressure_right(&self) -> &FsrFoot {
        &self.right.min
    }
}

/// System to initialize the [`FsrCalibration`] after loading the required configs.
fn init_fsr_calibration(mut commands: Commands, config: Res<SensorConfig>) {
    commands.insert_resource(CalibratedFsr::init(&config.fsr));
}

/// System that updates the [`FsrCalibration`] struct using sensor values.
fn update_fsr_calibration(
    config: Res<SensorConfig>,
    mut calibration: ResMut<CalibratedFsr>,
    fsr: Res<Fsr>,
) {
    calibration.left.update(&fsr.left_foot);
    calibration.right.update(&fsr.right_foot);

    calibration.max_pressure = calibration
        .max_pressure
        .max(fsr.left_foot.sum().max(fsr.right_foot.sum()));

    if calibration.last_updated.elapsed() >= config.fsr.highest_pressure_update_rate {
        calibration.last_updated = Instant::now();

        calibration
            .left
            .recalibrate_max_pressure(config.fsr.min_pressure_foot());
        calibration
            .right
            .recalibrate_max_pressure(config.fsr.min_pressure_foot());
    }

    calibration.update_foot_pressure(&fsr);
}

/// System that updates the minimum pressure value for each sensor, once the required number of foot
/// switches has occurred.
fn update_min_pressure(
    config: Res<SensorConfig>,
    mut calibration: ResMut<CalibratedFsr>,
    mut num_foot_switches: Local<u32>,
) {
    *num_foot_switches += 1;

    if *num_foot_switches < config.fsr.num_foot_switches {
        return;
    }

    calibration
        .left
        .recalibrate_min_pressure(config.fsr.max_pressure_foot());
    calibration
        .right
        .recalibrate_min_pressure(config.fsr.max_pressure_foot());

    // we encountered enough foot switches, we can safely update the minimum
    // we should only trust the calibration if we've calibrated the minimum pressure values.
    *num_foot_switches = 0;
    calibration.is_calibrated = true;
}
