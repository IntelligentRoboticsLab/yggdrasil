use miette::miette;
use odal::Config;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{nao::RobotInfo, prelude::*};

/// This config store general information for matches, for example things like
/// team number and player numbers.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShowtimeConfig {
    /// Team number to use during a match
    pub team_number: u8,
    /// This field contains mappings from robot ids to player numbers, the
    /// key is a String to make sure default serialization works
    pub robot_numbers_map: HashMap<String, u8>,
}

impl Config for ShowtimeConfig {
    const PATH: &'static str = "generated/showtime.toml";
}

/// This config store robot specificinformation for matches, for example
/// things like the robot team and player number.
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
    let player_number = *showtime_config
        .robot_numbers_map
        .get(&robot_info.robot_id.to_string())
        .ok_or(miette!(format!(
            "Could not find robot {} in showtime config",
            robot_info.robot_id
        )))?;
    let team_number = showtime_config.team_number;

    let player_config = PlayerConfig {
        player_number,
        team_number,
    };
    storage.add_resource(Resource::new(player_config))
}
