use miette::{miette, IntoDiagnostic, Report, Result};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use tokio::process::{Child, Command};

/// Configuration structure for sindri (.toml), containing team number and robot information
#[serde_as]
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// The configured team number, used to construct IPs.
    pub team_number: u8,

    /// A mapping of robot numbers to their corresponding robot details
    #[serde_as(as = "HashMap<DisplayFromStr, _>")]
    pub robots: HashMap<u8, Robot>,
}

impl Config {
    /// Retrieve the name of a robot based on its number
    ///
    /// This will return `Robot number not found!` if the robot's name hasn't been configured yet!
    pub fn get_robot_name(&self, number: u8) -> Result<&str, Report> {
        self.robots
            .get(&number)
            .map(|robot| robot.name.as_str())
            .ok_or_else(|| miette!("Robot number not found!"))
    }
}

impl Default for Config {
    fn default() -> Self {
        let mut robots = HashMap::with_capacity(7);

        // Insert all DNT robots here as default values
        robots.insert(20, Robot::new("sam", 20));
        robots.insert(21, Robot::new("moos", 21));
        robots.insert(22, Robot::new("phineas", 22));
        robots.insert(23, Robot::new("ferb", 23));
        robots.insert(24, Robot::new("momo", 24));
        robots.insert(25, Robot::new("appa", 25));
        robots.insert(26, Robot::new("daphne", 26));

        Self {
            team_number: 8,
            robots,
        }
    }
}

/// Struct representing a robot with its name and number
#[derive(Debug, Deserialize, Clone)]
pub struct Robot {
    pub name: String,
    pub number: u8,
}

impl Robot {
    pub fn new(name: impl AsRef<str>, number: u8) -> Self {
        Self {
            name: name.as_ref().to_string(),
            number,
        }
    }

    /// Create an Ipv4 address for the robot based on robot number, team number and wired/wireless
    #[must_use]
    pub fn get_ip(&self, team_number: u8, wired: bool) -> Ipv4Addr {
        Ipv4Addr::new(10, u8::from(wired), team_number, self.number)
    }

    /// SSH into the robot and run the provided command.
    ///
    /// This will block the current thread!
    pub fn ssh(addr: String, command: String) -> Result<Child> {
        Command::new("ssh")
            .arg(format!("nao@{}", addr.clone()))
            .arg("-t")
            .args(command.split(' ').collect::<Vec<&str>>())
            .kill_on_drop(true)
            .spawn()
            .into_diagnostic()
    }
}
