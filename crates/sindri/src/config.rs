use miette::{miette, IntoDiagnostic, Report, Result};
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use tokio::process::Command;

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

/// Struct representing a robot with its name and number
#[derive(Debug, Deserialize, Clone)]
pub struct Robot {
    pub name: String,
    pub number: u8,
}

impl Robot {
    /// Create an Ipv4 address for the robot based on robot number, team number and wired/wireless
    #[must_use]
    pub fn get_ip(&self, team_number: u8, wired: bool) -> Ipv4Addr {
        Ipv4Addr::new(10, u8::from(wired), team_number, self.number)
    }

    /// SSH into the robot.
    pub async fn ssh(addr: String) -> Result<()> {
        let ssh_status = Command::new("ssh")
            .arg(format!("nao@{}", addr.clone()))
            .arg("~/yggdrasil")
            .spawn()
            .into_diagnostic()?
            .wait()
            .await
            .into_diagnostic()?;

        if !ssh_status.success() {
            return Err(miette!("Failed to ssh into the nao."));
        }

        Ok(())
    }
}
