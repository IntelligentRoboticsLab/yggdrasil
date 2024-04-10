use odal::Config;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, io::Write};

use crate::{nao::RobotInfo, prelude::*};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShowtimeConfig {
    // pub player: PlayerConfig,
    pub team_number: u8,
    pub robot_numbers_map: HashMap<String, u8>,
}

impl Config for ShowtimeConfig {
    const PATH: &'static str = "showtime.toml";
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PlayerConfig {
    pub player_number: u8,
    pub team_number: u8,
}

#[startup_system]
pub(super) fn configure_showtime(
    storage: &mut Storage,
    showtime_config: &ShowtimeConfig,
    robot_info: &RobotInfo,
) -> Result<()> {
    let robot_id = &robot_info.robot_id.to_string();
    let player_number = *showtime_config.robot_numbers_map.get(robot_id).unwrap();
    let team_number = showtime_config.team_number;

    let mut f = std::fs::File::create("log.txt").unwrap();
    f.write_all(format!("{} {}\n", team_number, player_number).as_bytes())
        .unwrap();

    let player_config = PlayerConfig {
        player_number,
        team_number,
    };
    storage.add_resource(Resource::new(player_config))
}
