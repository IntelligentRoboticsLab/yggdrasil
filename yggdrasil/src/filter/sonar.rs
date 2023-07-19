use color_eyre::Result;
use nidhogg::{types::SonarValues, NaoState};
use tyr::prelude::*;

/// A module offering structured wrappers for sonar, derived from the raw [`NaoState`].
///
/// This module provides the following resources to the application:
/// - [`SonarValues`]
///
pub struct SonarFilter;

impl Module for SonarFilter {
    fn initialize(self, app: App) -> Result<App> {
        app.add_system(sonar_filter)
            .add_resource(Resource::new(SonarValues::default()))
    }
}

#[system]
fn sonar_filter(nao_state: &NaoState, sonar: &mut SonarValues) -> Result<()> {
    sonar.left = nao_state.sonar.left.clone();
    sonar.right = nao_state.sonar.right.clone();

    Ok(())
}
