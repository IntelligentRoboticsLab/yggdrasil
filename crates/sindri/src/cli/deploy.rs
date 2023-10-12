use clap::Parser;
use miette::{miette, IntoDiagnostic, Result};
use std::fs;
use tokio::process::Command;

use crate::{
    cargo,
    config::{Config, Robot},
};

const TARGET_PATH: &str = "x86_64-unknown-linux-gnu";
const RELEASE_PATH: &str = "./target/x86_64-unknown-linux-gnu/release/yggdrasil";
const DEPLOY_PATH: &str = "./deploy/yggdrasil";

#[derive(Clone, Debug, Parser)]
pub struct ConfigOptsDeploy {
    /// Robot number
    #[clap(long, short)]
    number: u8,

    /// Scan for wired (true) or wireless (false) robots [default: false]
    #[clap(long)]
    lan: bool,

    /// Team number [default: Set in `sindri_config.toml`]
    #[clap(long)]
    team_number: Option<u8>,
}

#[derive(Parser)]
#[clap(name = "deploy")]
pub struct Deploy {
    #[clap(flatten)]
    pub deploy: ConfigOptsDeploy,
}

impl Deploy {
    /// Constructs IP and deploys to the robot
    pub async fn deploy(self, config: Config) -> Result<()> {
        let addr = format!(
            "10.{}.{}.{}",
            u8::from(self.deploy.lan),
            self.deploy.team_number.unwrap_or(config.team_number),
            self.deploy.number
        );

        cargo::build("yggdrasil", true, Some(TARGET_PATH)).await?;
        fs::copy(RELEASE_PATH, DEPLOY_PATH).into_diagnostic()?;

        clone(addr.clone()).await?;
        Robot::ssh(addr.clone()).await?;

        Ok(())
    }
}

/// Copy the contents of the 'deploy' folder to the robot.
async fn clone(addr: String) -> Result<()> {
    println!("Cloning into the nao.");

    let clone = Command::new("scp")
        .arg("-r")
        .arg("./deploy/.")
        .arg(format!("nao@{}:~/", addr.clone()))
        .spawn()
        .into_diagnostic()?
        .wait()
        .await
        .into_diagnostic()?;

    if !clone.success() {
        return Err(miette!("Failed to secure copy to the nao."));
    }

    Ok(())
}
