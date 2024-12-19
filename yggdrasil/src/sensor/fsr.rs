use super::SensorConfig;
use crate::prelude::*;
use bevy::prelude::*;
use nidhogg::{types::ForceSensitiveResistors, NaoState};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use nalgebra::SVector;
use crate::sensor::low_pass_filter::LowPassFilter;

use std::fs;
use std::io::Write;

const TIMEOUT: f32 = 0.00;
// The sensor data ranges between 0.5 and 1.5. Therefore, value X is chosen.
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
}

/// Struct containing the various contact points of the Nao.
#[derive(Resource)]
pub struct Contacts {
    /// Whether the Nao is on the ground.
    pub ground: bool,
    // Whether pressure has been detected in the previous cycle.
    pub last_pressure: bool,
    // Timestamp of detected change in pressure state.
    pub last_switched: Option<Instant>,
    // The first-order Butterworth low-pass filter to apply to the FSR sensor data.
    pub lpf: LowPassFilter<1>,
    pub debug_file: std::fs::File, // Remove later
}

impl Default for Contacts {
    fn default() -> Self {
        Contacts {
            ground: false, // Maybe true easier, as we assume its on the ground?
            last_pressure: false,
            last_switched: None, // Maybe initialize already?
            lpf: LowPassFilter::new(OMEGA), // Not sure what this value should be, maybe with cuttoff freq..
            // lpf: LowPassFilter::with_cutoff_freq(FREQUENCY, CYCLES_PER_SEC),
            debug_file: fs::File::options().write(true).create(true).open("contacts.txt").unwrap(), // remove later
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

    // Check pressure state
    println!("Frequences avg: {}", nao_state.force_sensitive_resistors.avg());
    let fsr_vector = SVector::<f32, 1>::from([nao_state.force_sensitive_resistors.avg()]);
    let filtered_fsr = contacts.lpf.update(fsr_vector);
    println!("Filtered value: {}", filtered_fsr);
    println!("X: {}", filtered_fsr.x);
    let current_pressure = filtered_fsr.x > config.fsr.ground_contact_threshold;
    println!("Current pressure {}", current_pressure);

    // If change in pressure state has been detected
    if current_pressure != contacts.last_pressure {
        contacts.last_switched = Some(Instant::now());
    }

    let avg = nao_state.force_sensitive_resistors.avg();
    let value = filtered_fsr.x;

    writeln!(contacts.debug_file, "{},{}", avg, value).unwrap();
 
    if let Some(last_switched) = contacts.last_switched {
        // Only update the ground state if timeout duration has elapsed.
        if last_switched.elapsed() >= Duration::from_secs_f32(TIMEOUT) {
            contacts.ground = current_pressure;
        }
    } else {
        // Set ground contact for first initialization. (Can maybe be removed if initializing is different)
        contacts.ground = current_pressure;
        contacts.last_pressure = current_pressure;
        contacts.last_switched = Some(Instant::now());
    }

    contacts.last_pressure = current_pressure;

}
