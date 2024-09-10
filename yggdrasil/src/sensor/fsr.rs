use super::FilterConfig;
use crate::prelude::*;
use bevy::prelude::*;
use nidhogg::{types::ForceSensitiveResistors, NaoState};

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
    config: Res<FilterConfig>,
) {
    force_sensitive_resistors.left_foot = nao_state.force_sensitive_resistors.left_foot.clone();
    force_sensitive_resistors.right_foot = nao_state.force_sensitive_resistors.right_foot.clone();

    contacts.ground =
        nao_state.force_sensitive_resistors.avg() > config.fsr.ground_contact_threshold;
}
