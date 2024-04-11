use crate::{cli::robot_ops::change_single_network, config::SindriConfig};
use clap::Parser;
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

        change_single_network(robot, self.network).await?;
        Ok(())
    }
}
