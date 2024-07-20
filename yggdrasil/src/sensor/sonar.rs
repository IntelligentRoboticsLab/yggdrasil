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
            .add_system(sonar_dump.after(sonar_sensor))
            .init_resource::<SonarData>()?
            .init_resource::<SonarValues>()
    }
}

#[system]
fn sonar_sensor(nao_state: &NaoState, sonar: &mut SonarValues) -> Result<()> {
    sonar.left = nao_state.sonar.left;
    sonar.right = nao_state.sonar.right;

    Ok(())
}

struct SonarData(std::fs::File);

impl Default for SonarData {
    fn default() -> Self {
        Self(std::fs::File::create("sonar-data.csv").unwrap())
    }
}

#[system]
fn sonar_dump(data: &mut SonarData, sonar: &mut SonarValues) -> Result<()> {
    use std::io::Write;

    let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    writeln!(data.0, "{},{},{}", timestamp, sonar.left, sonar.right).unwrap();

    Ok(())
}
