use miette::Result;
use nidhogg::{types::ForceSensitiveResistors, NaoState};
use tyr::prelude::*;

/// A module offering the Force Sensitive Resistor (FSR) sensor data of the Nao, derived from the raw [`NaoState`].
///
/// By allowing systems to depend only on the FSR data, this design enhances the dependency graph's efficiency.
///
/// This module provides the following resources to the application:
/// - [`ForceSensitiveResistors`]
pub struct FSRFilter;

impl Module for FSRFilter {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(force_sensitive_resistor_filter)
            .add_resource(Resource::new(ForceSensitiveResistors::default()))
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
