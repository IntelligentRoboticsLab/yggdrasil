use clap::Parser;
use miette::{miette, IntoDiagnostic, Result};
use std::fs;
use tokio::process::Command;

use crate::{cargo, config::SindriConfig};

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
    pub async fn deploy(self, sindri_config: SindriConfig) -> Result<()> {
        let addr = format!(
            "10.{}.{}.{}",
            u8::from(self.deploy.lan),
            self.deploy.team_number.unwrap_or(sindri_config.team_number),
            self.deploy.number
        );

        cargo::build("yggdrasil", true, Some("x86_64-unknown-linux-gnu")).await?;
        fs::copy(
            "./target/x86_64-unknown-linux-gnu/release/yggdrasil",
            "./deploy/yggdrasil",
        )
        .into_diagnostic()?;

        clone(addr.clone()).await?;
        ssh(addr.clone()).await?;

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

/// SSH into the robot.
async fn ssh(addr: String) -> Result<()> {
    let ssh_status = Command::new("ssh")
        .arg(format!("nao@{}", addr.clone()))
        .arg("~/yggdrasil")
        .spawn()
        .into_diagnostic()?
        .wait()
        .await
        .into_diagnostic()?;

    if !ssh_status.success() {
        return Err(miette!("Failed to ssh into the nao."));
    }

    Ok(())
}
