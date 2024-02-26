use serde::{Deserialize, Serialize};

use crate::prelude::*;
use crate::{
    camera::CameraConfig, filter::FilterConfig, game_controller::GameControllerConfig,
    primary_state::PrimaryStateConfig,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct YggdrasilConfig {
    pub camera: CameraConfig,
    pub filter: FilterConfig,
    pub game_controller: GameControllerConfig,
    pub primary_state: PrimaryStateConfig,
}

impl Config for YggdrasilConfig {
    const PATH: &'static str = "yggdrasil.toml";
}
