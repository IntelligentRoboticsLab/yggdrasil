use std::collections::HashMap;
use std::time::Duration;

use clap::Parser;
use colored::Colorize;
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use miette::{miette, IntoDiagnostic, Result};
use tokio::runtime::Handle;

use crate::{
    cli::robot_ops::{ConfigOptsRobotOps, RobotEntry},
    config::SindriConfig,
};
use yggdrasil::{core::config::showtime::ShowtimeConfig, prelude::Config as OdalConfigTrait};

use super::robot_ops;

const DEFAULT_PLAYER_NUMBER: u8 = 3;
const DEFAULT_TEAM_NUMBER: u8 = 8;

/// Compile, deploy and run the specified binary on multiple robots, with the option of setting
/// player numbers.
#[derive(Parser, Debug)]
pub struct Showtime {
    #[clap(flatten)]
    pub robot_ops: ConfigOptsRobotOps,
}

impl Showtime {
    /// This command compiles yggdrasil, stops the yggdrasil service on each robot
    /// uploads binaries and other assets and the restarts the yggdrasil service
    /// on each robot.
    pub async fn showtime(self, config: SindriConfig) -> Result<()> {
        let mut robot_assignments = HashMap::new();
        for RobotEntry {
            robot_number,
            player_number,
        } in self.robot_ops.robots.iter()
        {
            if let Some(player_number) = player_number {
                robot_assignments.insert((*robot_number).to_string(), *player_number);
            } else {
                robot_assignments.insert((*robot_number).to_string(), DEFAULT_PLAYER_NUMBER);
            }
        }
        let showtime_config = ShowtimeConfig {
            team_number: self.robot_ops.team.unwrap_or(DEFAULT_TEAM_NUMBER),
            robot_numbers_map: robot_assignments,
        };

        // Store the config
        showtime_config
            .store("./deploy/config/generated/showtime.toml")
            .map_err(|e| {
                miette!(format!(
                    "{e} Make sure you run Yggdrasil from the root of the project"
                ))
            })?;

        let compile_bar = ProgressBar::new(1);
        let output = robot_ops::Output::Single(compile_bar.clone());
        robot_ops::compile(self.robot_ops.clone(), output.clone()).await?;

        if self.robot_ops.robots.len() == 1 {
            let output = robot_ops::Output::Single(compile_bar.clone());
            let robot = config
                .robot(
                    self.robot_ops.robots.first().unwrap().robot_number,
                    self.robot_ops.wired,
                )
                .unwrap();

            output.spinner();
            robot_ops::stop_single_yggdrasil_service(&robot, output.clone()).await?;
            robot_ops::upload_to_robot(&robot.ip(), output.clone()).await?;
            output.spinner();
            robot_ops::start_single_yggdrasil_service(&robot, output.clone()).await?;

            if let Some(network) = self.robot_ops.network {
                output.spinner();
                robot_ops::change_single_network(&robot, network, output.clone()).await?;
            }

            output.finished_deploying(&robot.ip());
            return Ok(());
        }

        compile_bar.finish_and_clear();

        let mut join_set = tokio::task::JoinSet::new();

        let multi = MultiProgress::new();
        multi.set_alignment(indicatif::MultiProgressAlignment::Bottom);
        let deploy_bar = multi.add(
            ProgressBar::new_spinner().with_style(
                ProgressStyle::with_template(
                    "   {prefix:.blue.bold} to robots {msg} {spinner:.blue.bold}",
                )
                .unwrap(),
            ),
        );
        deploy_bar.enable_steady_tick(Duration::from_millis(80));
        deploy_bar.set_prefix("Deploying");
        deploy_bar.set_message(format!(
            "{}{}, {}{}{}",
            "(network: ".dimmed(),
            self.robot_ops
                .network
                .clone()
                .unwrap_or("None".to_string())
                .bright_yellow(),
            "robots: ".dimmed(),
            self.robot_ops.robots.len().to_string().bold(),
            ")".dimmed()
        ));

        for robot in self.robot_ops.robots.iter() {
            let robot = config
                .robot(robot.robot_number, self.robot_ops.wired)
                .unwrap();
            let multi = multi.clone();
            let network = self.robot_ops.network.clone();

            join_set.spawn_blocking(move || {
                let multi = multi.clone();
                let handle = Handle::current();
                let pb = ProgressBar::new(1);
                let pb = multi.add(pb);
                let output = robot_ops::Output::Multi(pb);

                handle
                    .block_on(async move {
                        output.spinner();
                        robot_ops::stop_single_yggdrasil_service(&robot, output.clone()).await?;
                        robot_ops::upload_to_robot(&robot.ip(), output.clone()).await?;
                        output.spinner();
                        robot_ops::start_single_yggdrasil_service(&robot, output.clone()).await?;

                        if let Some(network) = network {
                            output.spinner();
                            robot_ops::change_single_network(&robot, network, output.clone())
                                .await?;
                        }

                        output.finished_deploying(&robot.ip());
                        Ok::<(), crate::error::Error>(())
                    })
                    .into_diagnostic()
            });
        }

        while let Some(result) = join_set.join_next().await {
            result.into_diagnostic()??;
        }
        deploy_bar.finish();
        println!(
            "{} in {}",
            "    Finished".cyan().bold(),
            HumanDuration(deploy_bar.elapsed()),
        );

        Ok(())
    }
}
