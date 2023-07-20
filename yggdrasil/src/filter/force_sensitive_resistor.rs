use color_eyre::Result;
use nidhogg::{
    types::{ForceSensitiveResistorFoot, ForceSensitiveResistors},
    NaoState,
};
use tyr::prelude::*;

/// A module offering the Force Sensitive Resistor (FSR) sensor data of the Nao, derived from the raw [`NaoState`].
///
/// By allowing systems to depend only on the FSR data, this design enhances the dependency graph's efficiency.
///
/// This module provides the following resources to the application:
/// - [`ForceSensitiveResistors`]
///
/// These resources include a [`ForceSensitiveResistorFoot`], containing the sensor values for each foot.
pub struct ForceSensitiveResistorFilter;

impl Module for ForceSensitiveResistorFilter {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(force_sensitive_resistor_filter)
            .add_resource(Resource::new(ForceSensitiveResistors::default()))?
            .add_resource(Resource::new(ForceSensitiveResistorFoot::default()))
    }
}

#[system]
fn force_sensitive_resistor_filter(
    nao_state: &NaoState,
    force_sensitive_resistors: &mut ForceSensitiveResistors,
) -> Result<()> {
    force_sensitive_resistors.left_foot = nao_state.force_sensitive_resistors.left_foot.clone();
    force_sensitive_resistors.right_foot = nao_state.force_sensitive_resistors.right_foot.clone();

    Ok(())
}
