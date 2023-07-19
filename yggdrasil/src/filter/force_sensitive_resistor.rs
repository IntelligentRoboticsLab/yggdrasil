use color_eyre::Result;
use nidhogg::{
    types::{ForceSensitiveResistorFoot, ForceSensitiveResistors},
    NaoState,
};
use tyr::prelude::*;

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
