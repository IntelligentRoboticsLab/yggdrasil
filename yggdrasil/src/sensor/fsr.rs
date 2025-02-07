use super::SensorConfig;
use crate::prelude::*;
use crate::sensor::low_pass_filter::LowPassFilter;
use bevy::prelude::*;
use nalgebra::SVector;
use nidhogg::{types::ForceSensitiveResistors, NaoState};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

// Omega for the low-pass filter.
const OMEGA: f32 = 0.7;

/// Plugin offering the Force Sensitive Resistor (FSR) sensor data of the Nao,
/// derived from the raw [`NaoState`].
pub struct FSRSensorPlugin;

impl Plugin for FSRSensorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Sensor, force_sensitive_resistor_sensor)
            .init_resource::<ForceSensitiveResistors>()
            .init_resource::<Contacts>();
    }
}

/// Configuration for the ground contact detection using FSR.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FsrConfig {
    /// Threshold for ground contact detection using average FSR sensor values from both feet.
    pub ground_contact_threshold: f32,
    /// Timeout for change of value of the ground contact state in seconds.
    pub ground_contact_timeout: f32,
}

/// Struct containing the various contact points of the Nao.
#[derive(Resource)]
pub struct Contacts {
    /// Whether the Nao is on the ground.
    pub ground: bool,
    /// Whether pressure has been detected in the previous cycle.
    pub last_pressure: bool,
    /// Timestamp of detected change in pressure state.
    pub last_switched: Instant,
    /// The first-order Butterworth low-pass filter to apply to the FSR sensor data.
    pub lpf: LowPassFilter<1>,
}

impl Default for Contacts {
    fn default() -> Self {
        Contacts {
            ground: true,
            last_pressure: true,
            last_switched: Instant::now(),
            lpf: LowPassFilter::new(OMEGA),
        }
    }
}

fn force_sensitive_resistor_sensor(
    nao_state: Res<NaoState>,
    mut force_sensitive_resistors: ResMut<ForceSensitiveResistors>,
    mut contacts: ResMut<Contacts>,
    config: Res<SensorConfig>,
) {
    force_sensitive_resistors.left_foot = nao_state.force_sensitive_resistors.left_foot.clone();
    force_sensitive_resistors.right_foot = nao_state.force_sensitive_resistors.right_foot.clone();

    // Retrieve FSR values and apply low-pass filter.
    let fsr_vector = SVector::<f32, 1>::from([nao_state.force_sensitive_resistors.avg()]);
    let filtered_fsr = contacts.lpf.update(fsr_vector);
    let current_pressure = filtered_fsr.x > config.fsr.ground_contact_threshold;

    if current_pressure != contacts.last_pressure {
        contacts.last_switched = Instant::now();
    }

    // Only update the ground state if timeout duration has elapsed.
    if contacts.last_switched.elapsed()
        >= Duration::from_secs_f32(config.fsr.ground_contact_timeout)
    {
        contacts.ground = current_pressure;
    }

    contacts.last_pressure = current_pressure;
}
