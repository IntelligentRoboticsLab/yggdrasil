use clap::Parser;
use miette::{miette, ErrReport, IntoDiagnostic, Report, Result};
use std::collections::HashMap;
use tokio::{self, task::JoinHandle};

use crate::{cli::deploy::ConfigOptsDeploy, cli::deploy::Deploy, config::Config};
use yggdrasil::config::showtime::ShowtimeConfig;
use yggdrasil::prelude::Config as OdalConfigTrait;

/// Compile, deploy and run the specified binary to the robot.
#[derive(Parser, Debug)]
pub struct Showtime {
    /// The robot numbers of the robots to set in showtime
    #[clap(required = true)]
    pub robot_numbers: Vec<String>,
    #[clap(long, short)]
    pub wired: bool,
}

fn parse_map(robot_numbers: &Vec<String>) -> HashMap<u8, Option<u8>> {
    let mut robot_player_map: HashMap<u8, Option<u8>> = HashMap::new();

    for robot_number in robot_numbers {
        if robot_number.find(':').is_some() {
            let pair: Vec<&str> = robot_number.split(':').collect();
            robot_player_map.insert(pair[0].parse().unwrap(), Some(pair[1].parse().unwrap()));
        } else {
            robot_player_map.insert(robot_number.parse().unwrap(), None);
        }
    }
    robot_player_map
}

impl Showtime {
    pub async fn showtime(self, config: Config) -> Result<()> {
        let robot_numbers = parse_map(&self.robot_numbers);

        let mut showtime_config = ShowtimeConfig::load("./deploy/config/")?;

        // Alter the robot id to player number map if needed
        for (robot_id, player_number) in robot_numbers.clone().into_iter() {
            // If player number is Some update map
            if let Some(player_number) = player_number {
                if let Some(old_player_number) = showtime_config
                    .robot_numbers_map
                    .get_mut(&robot_id.to_string())
                {
                    *old_player_number = player_number;
                }
            }
        }

        showtime_config.store("./deploy/config/showtime.toml")?;

        // Deploy and start yggdrasil on all chosen robots simultaneously
        let mut threads: Vec<JoinHandle<Result<(), Report>>> = vec![];
        for (robot_number, _) in robot_numbers.clone().into_iter() {
            let temp_config = config.clone();
            let thread = tokio::spawn(async move {
                let robot = temp_config
                    .robot(robot_number, self.wired)
                    .ok_or(miette!(format!(
                        "Invalid robot specified, number {} is not configured!",
                        robot_number
                    )))?;

                let envs: Vec<(&str, &str)> = Vec::new();

                robot
                    .ssh("sudo systemctl stop yggdrasil", envs.clone(), true)?
                    .wait()
                    .await
                    .into_diagnostic()?;

                Deploy {
                    deploy: ConfigOptsDeploy::new(
                        robot_number,
                        self.wired,
                        Some(config.team_number),
                        false,
                        false,
                        true,
                        String::from("yggdrasil"),
                        true,
                    ),
                }
                .deploy(temp_config.clone())
                .await?;

                robot
                    .ssh("sudo systemctl restart yggdrasil", envs, true)?
                    .wait()
                    .await
                    .into_diagnostic()?;
                println!("Yggdrasil started on robot: {}", robot_number);

                Ok::<(), ErrReport>(())
            });
            threads.push(thread);
        }

        for temp_thread in threads {
            temp_thread.await.unwrap()?;
        }

        Ok(())
    }
}
