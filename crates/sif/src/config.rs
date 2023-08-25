use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;
use std::net::Ipv4Addr;

/// Configuration structure for sif (.toml), containing team number and robot information
#[serde_as]
#[derive(Debug, Deserialize, Clone)]
pub struct SifConfig {
    pub team_number: u8,

     /// A mapping of robot numbers to their corresponding robot details
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub robots: HashMap<u8, Robot>,
}

impl SifConfig {
    /// Retrieve the name of a robot based on its number
    pub fn get_robot_name(&self, number: u8) -> String {
        self.robots
            .get(&number)
            .map(|robot| robot.name.to_string())
            .unwrap_or("unknown".to_string())
    }
}

/// Struct representing a robot with its name and number
#[derive(Debug, Deserialize, Clone)]
pub struct Robot {
    pub name: String,
    pub number: u8,
}

impl Robot {
    /// Create an Ipv4 address for the robot based on robot number, team number and wired/wireless
    pub fn get_ip(&self, team_number: u8, wired: bool) -> Ipv4Addr {
        Ipv4Addr::new(
            10,
            if wired { 1 } else { 0 },
            team_number,
            self.number,
        )
    }
}
