use bevy::prelude::*;
use odal::Config;
use serde::{Deserialize, Serialize};

// use crate::sensor::orientation::OrientationFilterConfig;
// use game_controller::GameControllerConfig;
// use vision::camera::CameraConfig;
// use crate::{behavior::primary_state::PrimaryStateConfig, sensor::SensorConfig};

#[derive(Resource, Debug, Deserialize, Serialize)]
// #[serde(deny_unknown_fields)]
pub struct YggdrasilConfig {
    // pub camera: CameraConfig,
    // pub filter: SensorConfig,
    // pub game_controller: GameControllerConfig,
    // pub primary_state: PrimaryStateConfig,
    // TODO: Add this back whenever we have something again
    // pub vision: VisionConfig,
    // pub orientation: OrientationFilterConfig,
}

impl Config for YggdrasilConfig {
    const PATH: &'static str = "yggdrasil.toml";
}
