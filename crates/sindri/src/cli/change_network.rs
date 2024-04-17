use std::time::Duration;

use crate::{
    cli::robot_ops::{self, change_single_network},
    config::SindriConfig,
};
use clap::Parser;
use colored::Colorize;
use indicatif::HumanDuration;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;
use miette::miette;
use miette::Result;
/// Changes the default network the robot connects to.
#[derive(Parser, Debug)]
pub struct ChangeNetwork {
    #[clap(long, short)]
    pub network: String,
    #[clap(long, short)]
    pub robot: u8,
    #[clap(long, short)]
    pub wired: bool,
}

impl ChangeNetwork {
    pub async fn change_network(self, config: SindriConfig) -> Result<()> {
        let robot = config.robot(self.robot, self.wired).ok_or(miette!(format!(
            "Invalid robot specified, number {} is not configured!",
            self.robot
        )))?;

        let pb = ProgressBar::new_spinner().with_style(
            ProgressStyle::with_template("   {prefix:.magenta.bold} {msg} {spinner:.magenta}")
                .unwrap(),
        );
        pb.enable_steady_tick(Duration::from_millis(80));
        pb.set_prefix("Updating");
        pb.set_message(format!(
            "{} {}",
            "network to".bold(),
            self.network.bright_yellow()
        ));
        let output = robot_ops::Output::Single(pb.clone());
        change_single_network(&robot, self.network, output).await?;
        pb.finish();
        println!(
            "    {} in {}",
            "Updated".magenta().bold(),
            HumanDuration(pb.elapsed()),
        );

        Ok(())
    }
}
