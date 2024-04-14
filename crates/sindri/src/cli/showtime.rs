use clap::Parser;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use miette::{miette, IntoDiagnostic, Result};
use tokio::task::JoinSet;

use crate::{
    cli::robot_ops::{ConfigOptsRobotOps, Output, RobotEntry, RobotOps},
    config::SindriConfig,
};
use yggdrasil::{config::showtime::ShowtimeConfig, prelude::Config as OdalConfigTrait};

use super::robot_ops;

/// Compile, deploy and run the specified binary on multiple robots, with the option of setting
/// player numbers.
#[derive(Parser, Debug)]
pub struct Showtime {
    #[clap(flatten)]
    pub config: ConfigOptsRobotOps,
}

impl Showtime {
    /// This command compiles yggdrasil, stops the yggdrasil service on each robot
    /// uploads binaries and other assets and the restarts the yggdrasil service
    /// on each robot.
    pub async fn showtime(self, config: SindriConfig) -> Result<()> {
        let mut showtime_config = ShowtimeConfig::load("./deploy/config/")
            .map_err(|_| miette!("Command must be executed from the yggdrasil directory"))?;

        // Alter the robot id to player number map if needed
        for RobotEntry {
            robot_number,
            player_number,
        } in self.config.robots.iter()
        {
            // If player number is Some update map
            if let Some(player_number) = player_number {
                if let Some(old_player_number) = showtime_config
                    .robot_numbers_map
                    .get_mut(&robot_number.to_string())
                {
                    *old_player_number = *player_number;
                }
            }
        }

        // Update the config
        showtime_config.store("./deploy/config/showtime.toml")?;

        // Initialize robot ops
        let ops = RobotOps {
            sindri_config: config.clone(),
            config: self.config.clone(),
        };

        ops.compile(Output::Single(ProgressBar::new_spinner()))
            .await?;

        if self.config.robots.len() == 1 {
            let robot = config
                .robot(self.config.robots.first().unwrap().robot_number, false)
                .unwrap();
            robot_ops::stop_single_yggdrasil_service(robot.clone()).await?;
            robot_ops::upload_to_robot(
                robot.ip(),
                robot_ops::SindriProgressBar::Single(ProgressBar::new(1)),
            )
            .await?;
            robot_ops::stop_single_yggdrasil_service(robot).await?;
            return Ok(());
        }

        let mut join_set = tokio::task::JoinSet::new();

        let multi = MultiProgress::new();

        for robot in self.config.robots.iter() {
            let pb = multi.add(ProgressBar::new(1));
            pb.set_style(
                ProgressStyle::with_template(
                    "{prefix:.blue.bold} [{bar:.green/cyan}]: {msg} {spinner:.cyan}",
                )
                .unwrap(),
            );
            pb.set_message(format!["Uploading to robot {}", robot.robot_number]);
            pb.tick();
            // multi.println("added pb").into_diagnostic()?;
            let robot = config.robot(robot.robot_number, false).unwrap();
            join_set.spawn(async move {
                // robot_ops::stop_single_yggdrasil_service(robot.clone()).await.unwrap();
                robot_ops::upload_to_robot(robot.ip(), robot_ops::Output::Multi(pb))
                    .await
                    .unwrap();
                // robot_ops::start_single_yggdrasil_service(robot).await;
            });
        }

        while let Some(result) = join_set.join_next().await {
            result.into_diagnostic()?;
        }

        // // ops.stop_yggdrasil_services().await?;
        // ops.upload(Output::Verbose).await?;
        // ops.start_yggdrasil_services().await?;
        // // ops.change_networks(self.config.network).await?;

        Ok(())
    }
}
