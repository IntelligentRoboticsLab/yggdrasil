use crate::prelude::*;
use nidhogg::{types::SonarValues, NaoState};

/// A module offering structured wrappers for sonar, derived from the raw [`NaoState`].
///
/// This module provides the following resources to the application:
/// - [`SonarValues`]
pub struct SonarSensor;

impl Module for SonarSensor {
    fn initialize(self, app: App) -> Result<App> {
        app.add_staged_system(SystemStage::Sensor, sonar_sensor)
            .init_resource::<SonarValues>()
    }
}

#[system]
fn sonar_sensor(nao_state: &NaoState, sonar: &mut SonarValues) -> Result<()> {
    sonar.left = nao_state.sonar.left;
    sonar.right = nao_state.sonar.right;

    Ok(())
}
