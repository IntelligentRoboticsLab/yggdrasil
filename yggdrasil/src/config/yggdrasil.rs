use serde::{Serialize, Deserialize};

use crate::prelude::*;
use crate::{camera::CameraConfig, filter::FilterConfig, game_controller::GameControllerConfig, primary_state::PrimaryStateConfig};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct YggdrasilConfig {
    camera: CameraConfig,
    filter: FilterConfig,
    game_controller: GameControllerConfig,
    primary_state: PrimaryStateConfig,
}

impl Config for YggdrasilConfig {
    const PATH: &'static str = "yggdrasil.toml";
}