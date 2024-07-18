use crate::{
    cli::robot_ops::{self, upload_to_robot},
    config::SindriConfig,
};
use clap::Parser;
use std::collections::HashMap;
use yggdrasil::{core::config::showtime::ShowtimeConfig, prelude::Config as OdalConfigTrait};
// use colored::Colorize;
use indicatif::ProgressBar;
// use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use miette::{miette, Context, IntoDiagnostic};

use super::robot_ops::RobotEntry;
// use std::fs;
// use std::time::Duration;

// const LOCAL_ROBOT_ID: u8 = 0;
const DEFAULT_PLAYER_NUMBER: u8 = 3;
const DEFAULT_TEAM_NUMBER: u8 = 8;

// const ROBOT_TARGET: &str = "x86_64-unknown-linux-gnu";
// const RELEASE_PATH: &str = "./target/x86_64-unknown-linux-gnu/release/skadi";
// const DEPLOY_PATH: &str = "./deploy/skadi";

#[derive(Clone, Debug, Parser)]
pub struct ConfigOptsRecord {
    /// Number of the robot to deploy to.
    #[clap(index = 1, name = "robot-number")]
    pub number: u8,

    /// Scan for wired (true) or wireless (false) robots [default: false]
    #[clap(short, long)]
    pub wired: bool,
}

impl ConfigOptsRecord {
    #[must_use]
    pub fn new(number: u8, wired: bool) -> Self {
        Self { number, wired }
    }
}

/// Utility for recording motions
#[derive(Parser)]
pub struct Record {
    #[clap(flatten)]
    pub record: ConfigOptsRecord,
}



impl Record {
    /// This procedure is a very similar procedure to sindri deploy, only instead of
    /// yggdrasil getting pushed skadi gets compiled and pushed to the robot.
    pub async fn record(self, config: SindriConfig) -> miette::Result<()> {
        let robots = vec![RobotEntry {
            robot_number: self.record.number,
            player_number: None,
        }];
        let compile_bar = ProgressBar::new(1);
        let output = robot_ops::Output::Single(compile_bar.clone());

        let ops = robot_ops::ConfigOptsRobotOps {
            bin: String::from("skadi"),
            team: Some(DEFAULT_TEAM_NUMBER),
            rerun: false,
            local: false,
            network: None,
            wired: self.record.wired,
            no_alsa: true,
            silent: false,
            robots,
        };

        robot_ops::compile(ops.clone(), output.clone()).await?;
        compile_bar.finish_and_clear();

        // Check if the robot exists
        let robot = config
            .robot(self.record.number, self.record.wired)
            .ok_or(miette!(format!(
                "Invalid robot specified, number {} is not configured!",
                self.record.number
            )))?;

        println!("{}", robot.ip());

        robot_ops::stop_single_yggdrasil_service(&robot, output).await?;

        // Generate showtime config
        let mut robot_assignments = HashMap::new();
        robot_assignments.insert(self.record.number.to_string(), DEFAULT_PLAYER_NUMBER);

        let showtime_config = ShowtimeConfig {
            team_number: DEFAULT_TEAM_NUMBER,
            robot_numbers_map: robot_assignments,
        };
        showtime_config
            .store("./deploy/config/generated/showtime.toml")
            .map_err(|e| {
                miette!(format!(
                    "{e} Make sure you run Yggdrasil from the root of the project"
                ))
            })?;

        let output = robot_ops::Output::Single(compile_bar.clone());
        upload_to_robot(&robot.ip(), output)
            .await
            .wrap_err("Failed to deploy yggdrasil files to robot")?;
        compile_bar.finish_and_clear();

        println!("Deze bitch klaar");

        robot
            .ssh("./skadi", Vec::<(&str, &str)>::new(), false)?
            .wait()
            .await
            .into_diagnostic()?;

        Ok(())
    }
}
