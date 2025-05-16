use std::time::Duration;

use crate::cli::robot_ops::NameOrNum;
use crate::{
    cli::robot_ops::{self, ShutdownCommand, shutdown_single_robot},
    config::SindriConfig,
};
use clap::Parser;
use colored::Colorize;
use futures::{TryStreamExt, stream::FuturesOrdered};
use indicatif::{HumanDuration, MultiProgress, ProgressBar, ProgressStyle};
use miette::miette;
use miette::{IntoDiagnostic, Result};
use tokio::runtime::Handle;

use super::scan;

/// Shuts down the robot
#[derive(Parser, Debug)]
pub struct Shutdown {
    #[clap(long, short)]
    pub wired: bool,
    #[clap(required_unless_present("all"))]
    pub robot_ids: Vec<NameOrNum>,
    #[clap(long, short)]
    pub restart: bool,
    #[clap(long, short)]
    pub team_number: Option<u8>,
    #[clap(long, short)]
    pub all: bool,
}

impl Shutdown {
    /// This command sends a signal to each robot to shutdown
    pub async fn shutdown(self, mut config: SindriConfig) -> Result<()> {
        if let Some(team_number) = self.team_number {
            config.team_number = team_number;
        }

        let kind = if self.restart {
            ShutdownCommand::Restart
        } else {
            ShutdownCommand::Shutdown
        };

        let multi = MultiProgress::new();
        multi.set_alignment(indicatif::MultiProgressAlignment::Bottom);
        let status_bar = multi.add(
            ProgressBar::new_spinner().with_style(
                ProgressStyle::with_template(
                    "   {prefix:.blue.bold} to robots {msg} {spinner:.blue.bold}",
                )
                .unwrap(),
            ),
        );

        status_bar.enable_steady_tick(Duration::from_millis(80));
        match kind {
            ShutdownCommand::Shutdown => status_bar.set_prefix("Shutdown signal"),
            ShutdownCommand::Restart => status_bar.set_prefix("Restart signal"),
        }
        status_bar.set_message(format!(
            "{}{}{}{}{}",
            "(robots: ".dimmed(),
            self.robot_ids.len().to_string().bold(),
            ", team number: ".dimmed(),
            config.team_number.to_string().bold(),
            ")".dimmed()
        ));

        let mut join_set = tokio::task::JoinSet::new();

        let robots = if self.all {
            config
                .robots
                .iter()
                .filter(|robot| robot.number != 0)
                .map(|robot_config| {
                    config
                        .robot(&NameOrNum::Number(robot_config.number), self.wired)
                        .unwrap()
                })
                .map(|robot| scan::ping(robot.ip()))
                .collect::<FuturesOrdered<_>>()
                .try_collect::<Vec<_>>()
                .await?
                .iter()
                .zip(&config.robots)
                .filter_map(|(scan_result, robot)| {
                    if scan_result.success() {
                        Some(NameOrNum::Number(robot.number))
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            self.robot_ids
        };

        for robot in robots {
            let robot = config.robot(&robot, self.wired).ok_or(miette!(format!(
                "Invalid robot specified, robot {robot} is not configured!"
            )))?;
            let multi = multi.clone();

            join_set.spawn_blocking(move || {
                let multi = multi.clone();
                let handle = Handle::current();
                let pb = ProgressBar::new(1);
                let pb = multi.add(pb);
                let output = robot_ops::Output::Multi(pb);

                handle
                    .block_on(async move {
                        output.spinner();
                        shutdown_single_robot(&robot, kind, output.clone()).await?;

                        output.finished_deploying(&robot.ip());
                        Ok::<(), crate::error::Error>(())
                    })
                    .into_diagnostic()
            });
        }

        while let Some(result) = join_set.join_next().await {
            result.into_diagnostic()??;
        }

        println!(
            "     {} in {}",
            "Shut down robot(s)".magenta().bold(),
            HumanDuration(status_bar.elapsed()),
        );
        Ok(())
    }
}
