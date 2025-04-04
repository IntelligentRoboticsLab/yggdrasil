use crate::cli::robot_ops::NameOrNum;
use crate::{
    cli::robot_ops::{self, shutdown_single_robot},
    config::SindriConfig,
};
use clap::Parser;
use colored::Colorize;
use indicatif::HumanDuration;
use indicatif::ProgressBar;
use miette::miette;
use miette::Result;

/// Shuts down the robot
#[derive(Parser, Debug)]
pub struct Shutdown {
    #[clap(long, short)]
    pub wired: bool,
    pub robot: u8,
    #[clap(long, short)]
    pub restart: bool,
}

impl Shutdown {
    /// This command sends a signal to each robot to shutdown
    pub async fn shutdown(self, config: SindriConfig) -> Result<()> {
        let robot = config
            .robot(&NameOrNum::Number(self.robot), self.wired)
            .ok_or(miette!(format!(
                "Invalid robot specified, number {} is not configured!",
                self.robot
            )))?;

        let pb = ProgressBar::new_spinner();
        let output = robot_ops::Output::Single(pb.clone());
        output.spinner();
        shutdown_single_robot(&robot, self.restart, output).await?;
        pb.finish();
        println!(
            "     {} in {}",
            "Shut down robot(s)".magenta().bold(),
            HumanDuration(pb.elapsed()),
        );

        Ok(())
    }
}
