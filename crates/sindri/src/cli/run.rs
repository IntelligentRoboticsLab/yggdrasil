use std::{os::unix::process::CommandExt, process::Stdio};

use clap::Parser;
use colored::Colorize;
use miette::{IntoDiagnostic, Result};
use tokio::process::Command;

use crate::{
    cli::robot_ops::{ConfigOptsRobotOps, RobotOps},
    config::SindriConfig,
};

use super::robot_ops::Output;

// TODO: refactor config for run
#[derive(Parser, Debug)]
/// Compile, deploy and run the specified binary to the robot.
pub struct Run {
    #[clap(flatten)]
    pub deploy: ConfigOptsRobotOps,
    /// Also print debug logs to stdout [default: false]
    #[clap(long, short)]
    pub debug: bool,
}

impl Run {
    /// Compiles, upload and then runs yggdrasil on one robot.
    /// This is done interactively so logs can be seen in the terminal.
    pub async fn run(self, config: SindriConfig) -> Result<()> {
        let local = self.deploy.local;
        let rerun = self.deploy.rerun;
        let has_rerun = has_rerun().await;

        if rerun && !has_rerun {
            println!(
                "{}: {}",
                "warning".bold().yellow(),
                "rerun is not installed, install it using `cargo install rerun-cli`".white()
            );
        }

        println!("{:?}", self.deploy.robots);

        let ops = RobotOps {
            sindri_config: config,
            config: self.deploy.clone(),
        };

        ops.compile(Output::Verbose).await?;

        if !self.deploy.local {
            ops.stop_yggdrasil_services().await?;
            ops.upload(Output::Verbose).await?;
        }

        let mut envs = Vec::new();
        if self.debug {
            envs.push(("RUST_LOG", "debug"));
        }

        if rerun {
            // Always set the host, so that rerun can connect to the correct host.
            // even if the host doesn't have rerun viewer installed, there could be
            // some case where the viewer is launched through a different method than the cli.
            let local_ip = local_ip_address::local_ip().into_diagnostic()?;
            envs.push(("RERUN_HOST", local_ip.to_string().leak()));

            if has_rerun {
                spawn_rerun_viewer()?;
            }
        }

        let robot = ops.get_first_robot()?;
        if local {
            robot
                .local("./yggdrasil", envs)?
                .wait()
                .await
                .into_diagnostic()?;
        } else {
            robot
                .ssh("./yggdrasil", envs, false)?
                .wait()
                .await
                .into_diagnostic()?;
        }

        Ok(())
    }
}

/// Check if the `rerun` binary is installed.
///
/// We check if the `rerun` binary is installed by running `rerun --version` and checking if the
/// command was successful.
async fn has_rerun() -> bool {
    async fn get_rerun_version() -> Result<bool> {
        Ok(Command::new("rerun")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .into_diagnostic()?
            .success())
    }

    get_rerun_version().await.is_ok_and(|success| success)
}

/// Spawn a rerun viewer in the background.
fn spawn_rerun_viewer() -> Result<()> {
    let mut process = std::process::Command::new("rerun");
    process
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);

    Command::from(process)
        .kill_on_drop(false)
        .spawn()
        .into_diagnostic()?;

    Ok(())
}
