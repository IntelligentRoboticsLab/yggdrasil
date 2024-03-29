use clap::Parser;
use miette::{miette, ErrReport, IntoDiagnostic, Report, Result};
use tokio::{self, task::JoinHandle};

use crate::{cli::deploy::ConfigOptsDeploy, cli::deploy::Deploy, config::Config};

/// Compile, deploy and run the specified binary to the robot.
#[derive(Parser, Debug)]
pub struct Pregame {
    /// The robot-numbers of the robots to pregame
    #[clap(required = true)]
    pub robot_numbers: Vec<u8>,
    #[clap(long, short)]
    pub wired: bool,
}

impl Pregame {
    pub async fn pregame(self, config: Config) -> Result<()> {
        println!("Robot numbers: {:?}", self.robot_numbers);
        let mut threads: Vec<JoinHandle<Result<(), Report>>> = vec![];
        for robot_number in self.robot_numbers {
            let temp_config = config.clone();
            let thread = tokio::spawn(async move {
                println!("Pregame robot: {}", robot_number);
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
                println!("Yggdrasil stopped on robot (if it was running): {}", robot_number);

                Deploy {
                    deploy: ConfigOptsDeploy::new(
                        robot_number,
                        self.wired,
                        Some(config.team_number),
                        false,
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
