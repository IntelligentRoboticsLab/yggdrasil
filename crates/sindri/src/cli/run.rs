use clap::Parser;
use miette::{miette, IntoDiagnostic, Result};

use crate::{
    cli::deploy::{ConfigOptsDeploy, Deploy},
    config::Config,
};

#[derive(Parser, Debug)]
/// Compile, deploy and run the specified binary to the robot.
pub struct Run {
    #[clap(flatten)]
    pub deploy: ConfigOptsDeploy,
    /// Also print debug logs to stdout [default: false]
    #[clap(long, short)]
    pub debug: bool,
}

impl Run {
    pub async fn run(self, config: Config) -> Result<()> {
        let robot = config
            .robot(self.deploy.number, self.deploy.wired)
            .ok_or(miette!(format!(
                "Invalid robot specified, number {} is not configured!",
                self.deploy.number
            )))?;

        Deploy {
            deploy: self.deploy,
        }
        .deploy(config)
        .await?;

        let mut envs = Vec::new();

        if self.debug {
            envs.push(("RUST_LOG", "debug"));
        };

        robot
            .ssh("./yggdrasil", envs)?
            .wait()
            .await
            .into_diagnostic()?;

        Ok(())
    }
}
