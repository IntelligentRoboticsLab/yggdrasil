use miette::{miette, Context, IntoDiagnostic, Result};
use serde::Deserialize;
use serde_with::serde_as;
use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::process::Stdio;
use std::{ffi::OsStr, net::Ipv4Addr};
use tokio::process::{Child, Command};

use crate::error::Error;

// Config location relative to home directory
const CONFIG_FILE: &str = ".config/sindri/sindri.toml";

pub fn config_file() -> PathBuf {
    home::home_dir()
        .expect("Failed to get home directory")
        .join(CONFIG_FILE)
}

pub fn load_config() -> Result<SindriConfig> {
    let config_file = config_file();

    let config_data = std::fs::read_to_string(config_file)
        .into_diagnostic()
        .wrap_err("Failed to read config file")?;

    toml::de::from_str(&config_data)
        .into_diagnostic()
        .wrap_err("Failed to parse config file!")
}

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
pub struct SindriConfig {
    /// The configured team number, used to construct IPs.
    pub team_number: u8,

    /// A [`Vec`] containing all robots configured in the configuration file.
    pub robots: Vec<ConfigRobot>,
}

impl<'a> SindriConfig {
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

    pub fn local<K, V>(
        &self,
        command: &str,
        envs: impl IntoIterator<Item = (K, V)>,
    ) -> Result<Child>
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let working_dir = format!(
            "{}/deploy/",
            std::env::current_dir().into_diagnostic()?.display()
        );

        Command::new(command)
            .current_dir(&working_dir)
            .envs(envs)
            .env("ROBOT_ID", self.number.to_string())
            .env("ROBOT_NAME", &self.name)
            .kill_on_drop(true)
            .spawn()
            .into_diagnostic()
    }

    /// SSH into the robot and run the provided command.
    ///
    /// This returns the spawned [`Child`] process.
    pub fn ssh<K, V>(
        &self,
        command: impl Into<String>,
        // Environment variables to run the command with
        remote_envs: impl IntoIterator<Item = (K, V)>,
        quiet: bool,
    ) -> crate::error::Result<Child>
    where
        K: AsRef<OsStr>,
        V: AsRef<OsStr>,
    {
        let remote_envs = remote_envs.into_iter().map(|(k, v)| {
            // k="v"
            let mut mapping = k.as_ref().to_os_string();
            mapping.push("=\"");
            mapping.push(v);
            mapping.push("\"");
            mapping
        });

        let mut quiet_arg = "";
        if !quiet {
            quiet_arg = "-t"
        }

        let command = command.into();
        Command::new("ssh")
            .arg("-o")
            .arg("StrictHostKeyChecking no")
            .arg(format!("nao@{}", self.ip()))
            .arg(quiet_arg)
            .args(remote_envs)
            .arg("bash -ilc")
            .arg(format!("\"{}\"", command.clone()))
            .kill_on_drop(true)
            .stderr(pick_stream(quiet))
            .stdout(pick_stream(quiet))
            .spawn()
            .map_err(|e| Error::Ssh { source: e, command })
    }
}

fn pick_stream(quiet: bool) -> Stdio {
    if quiet {
        Stdio::null()
    } else {
        Stdio::inherit()
    }
}
