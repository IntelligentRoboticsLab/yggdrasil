use clap::Parser;
use miette::{miette, ErrReport, IntoDiagnostic, Report, Result};
use std::fs::File;
use std::io::Read;
use std::{collections::HashMap, io::Write};
use tokio::{self, task::JoinHandle};

use crate::{cli::deploy::ConfigOptsDeploy, cli::deploy::Deploy, config::Config};
use yggdrasil::config::pregame::PregameConfig;

const PREGAME_CONFIG_PATH: &str = "./deploy/config/pregame.toml";

/// Compile, deploy and run the specified binary to the robot.
#[derive(Parser, Debug)]
pub struct Pregame {
    /// The robot-numbers of the robots to pregame
    #[clap(required = true)]
    pub robot_numbers: Vec<String>,
    #[clap(long, short)]
    pub wired: bool,
}

fn parse_map(robot_numbers: &Vec<String>) -> HashMap<u8, Option<u8>> {
    let mut robot_player_map: HashMap<u8, Option<u8>> = HashMap::new();

    for robot_number in robot_numbers {
        if let Some(_) = robot_number.find(":") {
            let pair: Vec<&str> = robot_number.split(":").collect();
            robot_player_map.insert(pair[0].parse().unwrap(), Some(pair[1].parse().unwrap()));
        } else {
            robot_player_map.insert(robot_number.parse().unwrap(), None);
        }
    }
    robot_player_map
}

impl Pregame {
    pub async fn pregame(self, config: Config) -> Result<()> {
        let robot_numbers = parse_map(&self.robot_numbers);

        // Read the Pregame config file
        let mut file = File::open(PREGAME_CONFIG_PATH).expect("Failed to open toml file");
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .expect("Failed to toml file");

        let mut pregame_config: PregameConfig = toml::from_str(&contents).into_diagnostic()?;

        // Alter the map if needed
        for (robot_id, player_number) in robot_numbers.clone().into_iter() {
            // If player number is Some update map
            if let Some(player_number) = player_number {
                if let Some(old_player_number) = pregame_config
                    .robot_numbers_map
                    .get_mut(&robot_id.to_string())
                {
                    *old_player_number = player_number;
                }
            }
        }

        // Write the new robot to player map to the toml
        let mut new_pregame_config = File::create(PREGAME_CONFIG_PATH).into_diagnostic()?;
        let pregame_data = toml::to_string(&pregame_config).into_diagnostic()?;
        let _ = new_pregame_config.write_all(pregame_data.as_bytes());

        // Pregame the selected robots simultaneously
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
                    .ssh("systemctl stop yggdrasil", envs.clone())?
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
                        "yggdrasil"
                    ),
                }
                .deploy(temp_config.clone())
                .await?;
                println!("Deployed on robot: {}", robot_number);

                robot
                    .ssh("systemctl start yggdrasil", envs)?
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
