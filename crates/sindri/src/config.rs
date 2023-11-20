use miette::{miette, IntoDiagnostic, Result};
use serde::Deserialize;
use serde_with::serde_as;
use std::net::Ipv4Addr;
use std::ops::RangeInclusive;
use tokio::process::{Child, Command};

/// Configuration structure for sindri (.toml), containing team number and robot information
#[serde_as]
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// The configured team number, used to construct IPs.
    pub team_number: u8,

    /// A [`Vec`] containing all robots configured in the configuration file.
    pub robots: Vec<Robot>,
}

impl Config {
    /// Retrieve the name of a robot based on its number
    ///
    /// This will return `Robot number not found!` if the robot's name hasn't been configured yet!
    pub fn robot_name(&self, number: u8) -> Result<&str> {
        self.by_number(number)
            .map(|r| r.name.as_str())
            .ok_or_else(|| miette!("Robot number not found!"))
    }

    /// Get a [`Robot`] instance using the provided number.
    ///
    /// If there's no [`Robot`] configured with the provided number, this will return an [`Option::None`].
    pub fn by_number(&self, number: u8) -> Option<&Robot> {
        self.robots.iter().find(|r| r.number == number)
    }

    /// Retrieve a range from the minimum robot number to the maximum robot number defined in this config.
    ///
    /// This range is fully inclusive for the minimum and maximum robot nubmer, e.g. [min, max]
    pub fn robot_range(&self) -> Result<RangeInclusive<u8>> {
        let min = self
            .robots
            .iter()
            .map(|r| r.number)
            .min()
            .ok_or(miette!("Faild to get minimum robot number!"))?;

        let max = self
            .robots
            .iter()
            .map(|r| r.number)
            .max()
            .ok_or(miette!("Failed to get maximum robot number!"))?;

        Ok(min..=max)
    }
}

/// Struct representing a robot with its name and number
#[derive(Debug, Deserialize, Clone)]
pub struct Robot {
    pub name: String,
    pub number: u8,
}

impl Robot {
    /// Creates a new [`Robot`] struct.
    pub fn new(name: impl Into<String>, number: u8) -> Self {
        Self {
            name: name.into(),
            number,
        }
    }

    /// Create an Ipv4 address for the robot based on robot number, team number and wired/wireless.
    #[must_use]
    pub fn ip(&self, team_number: u8, wired: bool) -> Ipv4Addr {
        Ipv4Addr::new(10, u8::from(wired), team_number, self.number)
    }

    /// SSH into the robot and run the provided command.
    ///
    /// This returns the spawned [`Child`] process.
    pub fn ssh(&self, team_number: u8, wired: bool, command: impl Into<String>) -> Result<Child> {
        Command::new("ssh")
            .arg(format!("nao@{}", self.ip(team_number, wired)))
            .arg("-t")
            .args(command.into().split(' ').collect::<Vec<&str>>())
            .kill_on_drop(true)
            .spawn()
            .into_diagnostic()
    }
}
