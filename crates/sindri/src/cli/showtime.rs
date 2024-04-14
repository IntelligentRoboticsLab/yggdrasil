use clap::Parser;
use miette::{miette, Result};

use crate::{
    cli::robot_ops::{ConfigOptsRobotOps, Output, RobotEntry, RobotOps},
    config::SindriConfig,
};
use yggdrasil::{config::showtime::ShowtimeConfig, prelude::Config as OdalConfigTrait};

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

        ops.compile(Output::Verbose).await?;
        ops.stop_yggdrasil_services().await?;
        ops.upload(Output::Verbose).await?;
        ops.start_yggdrasil_services().await?;
        // ops.change_networks(self.config.network).await?;

        Ok(())
    }
}
