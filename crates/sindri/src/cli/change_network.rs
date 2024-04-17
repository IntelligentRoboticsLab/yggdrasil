use crate::{
    cli::robot_ops::{self, change_single_network},
    config::SindriConfig,
};
use clap::Parser;
use colored::Colorize;
use indicatif::HumanDuration;
use indicatif::ProgressBar;
use miette::miette;
use miette::Result;

/// Changes the default network a specified robot connects to.
#[derive(Parser, Debug)]
pub struct ChangeNetwork {
    #[clap(long, short)]
    pub wired: bool,
    #[clap(long, short)]
    pub network: String,
    #[clap()]
    pub robot: u8,
}

impl ChangeNetwork {
    pub async fn change_network(self, config: SindriConfig) -> Result<()> {
        let robot = config.robot(self.robot, self.wired).ok_or(miette!(format!(
            "Invalid robot specified, number {} is not configured!",
            self.robot
        )))?;

        let pb = ProgressBar::new_spinner();
        let output = robot_ops::Output::Single(pb.clone());
        output.spinner();
        change_single_network(&robot, self.network, output).await?;
        pb.finish();
        println!(
            "     {} in {}",
            "Updated".magenta().bold(),
            HumanDuration(pb.elapsed()),
        );

        Ok(())
    }
}
