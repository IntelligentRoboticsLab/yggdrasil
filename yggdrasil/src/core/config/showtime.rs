use bevy::prelude::*;
use odal::Config;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::nao::RobotInfo;

/// This config store general information for matches, for example things like
/// team number and player numbers.
#[derive(Resource, Debug, Deserialize, Serialize)]
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
#[derive(Resource, Debug, Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PlayerConfig {
    pub player_number: u8,
    pub team_number: u8,
}

pub(super) fn configure_showtime(
    mut commands: Commands,
    showtime_config: Res<ShowtimeConfig>,
    robot_info: Res<RobotInfo>,
) {
    let player_number = *showtime_config
        .robot_numbers_map
        .get(&robot_info.robot_id.to_string())
        .expect(&format!(
            "Could not find robot {} in showtime config",
            robot_info.robot_id
        ));
    let team_number = showtime_config.team_number;

    let player_config = PlayerConfig {
        player_number,
        team_number,
    };

    commands.insert_resource(player_config);
}
