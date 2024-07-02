use crate::prelude::*;

use nidhogg::{types::ForceSensitiveResistors, NaoState};

use super::FilterConfig;

/// A module offering the Force Sensitive Resistor (FSR) sensor data of the Nao, derived from the raw [`NaoState`].
///
/// By allowing systems to depend only on the FSR data, this design enhances the dependency graph's efficiency.
///
/// This module provides the following resources to the application:
/// - [`ForceSensitiveResistors`]
pub struct FSRSensor;

impl Module for FSRSensor {
    fn initialize(self, app: App) -> Result<App> {
        app.add_staged_system(SystemStage::Sensor, force_sensitive_resistor_sensor)
            .init_resource::<ForceSensitiveResistors>()?
            .init_resource::<Contacts>()
    }
}

/// Struct containing the various contact points of the Nao.
#[derive(Default)]
pub struct Contacts {
    /// Whether the Nao is on the ground.
    pub ground: bool,
}

#[system]
pub fn force_sensitive_resistor_sensor(
    nao_state: &NaoState,
    force_sensitive_resistors: &mut ForceSensitiveResistors,
    contacts: &mut Contacts,
    config: &FilterConfig,
) -> Result<()> {
    force_sensitive_resistors.left_foot = nao_state.force_sensitive_resistors.left_foot.clone();
    force_sensitive_resistors.right_foot = nao_state.force_sensitive_resistors.right_foot.clone();

    contacts.ground =
        nao_state.force_sensitive_resistors.avg() > config.fsr.ground_contact_threshold;

    Ok(())
}
