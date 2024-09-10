use super::SensorConfig;
use crate::prelude::*;
use bevy::prelude::*;
use nidhogg::{types::ForceSensitiveResistors, NaoState};
use serde::{Deserialize, Serialize};

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
#[derive(Resource, Default)]
pub struct Contacts {
    /// Whether the Nao is on the ground.
    pub ground: bool,
}

fn force_sensitive_resistor_sensor(
    nao_state: Res<NaoState>,
    mut force_sensitive_resistors: ResMut<ForceSensitiveResistors>,
    mut contacts: ResMut<Contacts>,
    config: Res<SensorConfig>,
) {
    force_sensitive_resistors.left_foot = nao_state.force_sensitive_resistors.left_foot.clone();
    force_sensitive_resistors.right_foot = nao_state.force_sensitive_resistors.right_foot.clone();

    contacts.ground =
        nao_state.force_sensitive_resistors.avg() > config.fsr.ground_contact_threshold;
}
