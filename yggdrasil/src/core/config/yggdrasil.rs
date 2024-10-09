use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::game_controller::GameControllerConfig;
use crate::motion::odometry::OdometryConfig;
use crate::prelude::*;
use crate::sensor::orientation::OrientationFilterConfig;
use crate::vision::camera::CameraConfig;
use crate::{behavior::primary_state::PrimaryStateConfig, sensor::SensorConfig};

#[derive(Resource, Debug, Deserialize, Serialize)]
// #[serde(deny_unknown_fields)]
pub struct YggdrasilConfig {
    pub camera: CameraConfig,
    pub filter: SensorConfig,
    pub game_controller: GameControllerConfig,
    pub primary_state: PrimaryStateConfig,
    // TODO: Add this back whenever we have something again
    // pub vision: VisionConfig,
    pub odometry: OdometryConfig,
    pub orientation: OrientationFilterConfig,
}

impl Config for YggdrasilConfig {
    const PATH: &'static str = "yggdrasil.toml";
}
