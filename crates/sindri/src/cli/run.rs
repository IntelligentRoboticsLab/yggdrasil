use clap::Parser;
use miette::{miette, IntoDiagnostic, Result};

use crate::{
    cli::deploy::{ConfigOptsDeploy, Deploy},
    config::Config,
};

#[derive(Parser)]
/// Compile, deploy and run the specified binary to the robot.
pub struct Run {
    #[clap(flatten)]
    pub deploy: ConfigOptsDeploy,
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

        robot.ssh("./yggdrasil")?.wait().await.into_diagnostic()?;

        Ok(())
    }
}
