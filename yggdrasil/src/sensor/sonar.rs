use crate::prelude::*;
use bevy::prelude::*;
use nidhogg::{types::SonarValues, NaoState};

/// Plugin that offers a structured wrappers for sonar,
/// derived from the raw [`NaoState`].
pub struct SonarSensorPlugin;

impl Plugin for SonarSensorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Sensor, sonar_sensor)
            .init_resource::<SonarValues>()
    }
}

fn sonar_sensor(nao_state: Res<NaoState>, mut sonar: ResMut<SonarValues>) {
    sonar.left = nao_state.sonar.left;
    sonar.right = nao_state.sonar.right;
}
