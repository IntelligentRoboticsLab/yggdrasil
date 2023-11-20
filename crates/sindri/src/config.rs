use miette::{miette, IntoDiagnostic, Result};
use serde::Deserialize;
use serde_with::serde_as;
use std::net::Ipv4Addr;
use std::ops::RangeInclusive;
use tokio::process::{Child, Command};

/// A robot as defined in the sindri configuration
#[derive(Debug, Deserialize, Clone)]
pub struct ConfigRobot {
    pub name: String,
    pub number: u8,
}

impl ConfigRobot {
    #[must_use]
    pub fn to_robot(self, team_number: u8, wired: bool) -> Robot {
        Robot {
            name: self.name,
            number: self.number,
            team_number,
            wired,
        }
    }
}

/// Configuration structure for sindri (.toml), containing team number and robot information
#[serde_as]
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// The configured team number, used to construct IPs.
    pub team_number: u8,

    /// A [`Vec`] containing all robots configured in the configuration file.
    pub robots: Vec<ConfigRobot>,
}

impl<'a> Config {
    /// Get a [`Robot`] instance using the provided number.
    ///
    /// If there's no [`Robot`] configured with the provided number, this will return an [`Option::None`].
    pub fn robot(&'a self, number: u8, wired: bool) -> Option<Robot> {
        self.robots
            .iter()
            .find(|r| r.number == number)
            .cloned()
            .map(|c| c.to_robot(self.team_number, wired))
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
            .ok_or(miette!("Failed to get minimum robot number!"))?;

        let max = self
            .robots
            .iter()
            .map(|r| r.number)
            .max()
            .ok_or(miette!("Failed to get maximum robot number!"))?;

        Ok(min..=max)
    }
}

/// Struct representing a robot to which we can connect
#[derive(Debug, Deserialize, Clone)]
pub struct Robot {
    pub name: String,
    pub number: u8,
    pub team_number: u8,
    pub wired: bool,
}

impl Robot {
    pub fn new(name: impl Into<String>, number: u8, team_number: u8, wired: bool) -> Self {
        Self {
            name: name.into(),
            number,
            team_number,
            wired,
        }
    }

    /// Create an Ipv4 address for the robot based on robot number, team number and wired/wireless.
    #[must_use]
    pub fn ip(&self) -> Ipv4Addr {
        Ipv4Addr::new(10, u8::from(self.wired), self.team_number, self.number)
    }

    /// SSH into the robot and run the provided command.
    ///
    /// This returns the spawned [`Child`] process.
    pub fn ssh(&self, command: impl Into<String>) -> Result<Child> {
        Command::new("ssh")
            .arg(format!("nao@{}", self.ip()))
            .arg("-t")
            .args(command.into().split(' ').collect::<Vec<&str>>())
            .kill_on_drop(true)
            .spawn()
            .into_diagnostic()
    }
}
