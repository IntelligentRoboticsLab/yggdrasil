use serde::{Deserialize, Serialize};

use crate::motion::odometry::OdometryConfig;
use crate::prelude::*;
use crate::sensor::orientation::OrientationFilterConfig;
use crate::{
    behavior::primary_state::PrimaryStateConfig, game_controller::GameControllerConfig,
    sensor::FilterConfig, vision::camera::CameraConfig, vision::VisionConfig,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct YggdrasilConfig {
    pub camera: CameraConfig,
    pub filter: FilterConfig,
    pub game_controller: GameControllerConfig,
    pub primary_state: PrimaryStateConfig,
    pub vision: VisionConfig,
    pub odometry: OdometryConfig,
    pub orientation: OrientationFilterConfig,
}

impl Config for YggdrasilConfig {
    const PATH: &'static str = "yggdrasil.toml";
}
