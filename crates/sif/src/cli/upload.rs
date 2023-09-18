use clap::Parser;
use miette::{Result, IntoDiagnostic, miette};
use tokio::process::Command;
use std::fs;

use crate::{cargo, config::SifConfig};

#[derive(Clone, Debug, Parser)]
pub struct ConfigOptsUpload {
    /// Robot number
    #[clap(long, short)]
    number: u8,

    /// Scan for wired (true) or wireless (false) robots [default: false]
    #[clap(long)]
    lan: bool,

    /// Team number [default: Set in `sif_config.toml`]
    #[clap(long)]
    team_number: Option<u8>,
}

#[derive(Parser)]
#[clap(name = "upload")]
pub struct Upload {
    #[clap(flatten)]
    pub upload: ConfigOptsUpload,
}

impl Upload {
    /// Constructs IP and uploads to the robot
    pub async fn upload(self, sif_config: SifConfig) -> Result<()> {
        let addr = format!(
            "10.{}.{}.{}",
            u8::from(self.upload.lan),
            self.upload.team_number.unwrap_or(sif_config.team_number),
            self.upload.number
        );

        cargo::build("yggdrasil".to_owned(), true, Some("x86_64-unknown-linux-gnu".to_owned())).await?;
        fs::copy("./target/x86_64-unknown-linux-gnu/release/yggdrasil", "./deploy/yggdrasil").into_diagnostic()?;

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