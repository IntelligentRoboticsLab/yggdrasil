use odal::Config;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{nao::RobotInfo, prelude::*};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PregameConfig {
    // pub player: PlayerConfig,
    pub team_number: u8,
    pub robot_numbers_map: HashMap<String, u8>,
}

impl Config for PregameConfig {
    const PATH: &'static str = "pregame.toml";
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PlayerConfig {
    pub player_number: u8,
    pub team_number: u8,
}

#[startup_system]
pub(super) fn configure_pregame(
    storage: &mut Storage,
    pregame_config: &PregameConfig,
    robot_info: &RobotInfo,
) -> Result<()> {
    let robot_id = &robot_info.robot_id.to_string();
    let player_number = *pregame_config.robot_numbers_map.get(robot_id).unwrap();
    let team_number = pregame_config.team_number;

    let player_config = PlayerConfig {
        player_number,
        team_number,
    };
    storage.add_resource(Resource::new(player_config))
}
