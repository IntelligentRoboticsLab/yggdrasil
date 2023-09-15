use std::path::PathBuf;

use clap::Parser;
use miette::{miette, Result};
use rusync::{self, ConsoleProgressInfo, SyncOptions, Syncer};
use tokio::process::Command;

use crate::{cargo, config::SifConfig};

#[derive(Clone, Debug, Parser)]
pub struct ConfigOptsUpload {
    /// Robot number
    #[clap(long)]
    robot_number: u8,

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
            self.upload.robot_number
        );

        cargo::build("yggdrasil".to_owned(), true, Some("x86_64-unknown-linux-gnu".to_owned())).await?;

        let upload_target = format!("nao@{}:~/yggdrasil", addr.clone());
        println!("built target: {upload_target}");
        let syncer = Syncer::new(
            &PathBuf::from("/home/joost/Documents/GitHub/yggdrasil/target/x86_64-unknown-linux-gnu/release/yggdrasil"),
            &PathBuf::from(upload_target),
            SyncOptions::default(),
            Box::new(ConsoleProgressInfo::default()),
        );

        let synkie = syncer.sync().map_err(|_| miette!("oops"))?;
        println!("sync result: {synkie:?}");

        let ssh_status = Command::new("ssh")
            .arg(format!("nao@{}", addr.clone()))
            .arg("./yggdrasil")
            .status()
            .await;
        Ok(())
    }
}
