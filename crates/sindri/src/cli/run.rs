use std::process::Stdio;

use clap::Parser;
use colored::Colorize;
use miette::{miette, IntoDiagnostic, Result};
use tokio::process::Command;

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

        let local = self.deploy.local;
        let rerun = self.deploy.rerun;

        let has_rerun = has_rerun().await.is_ok_and(|success| success);

        if rerun && !has_rerun {
            println!(
                "{}: {}",
                "warning".bold().yellow(),
                "rerun is not installed, install it using `cargo install rerun-cli`".white()
            );
        }

        Deploy {
            deploy: self.deploy,
        }
        .deploy(config)
        .await?;

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

        if local {
            robot
                .local("./yggdrasil", envs)?
                .wait()
                .await
                .into_diagnostic()?;
        } else {
            robot
                .ssh("./yggdrasil", envs)?
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
async fn has_rerun() -> Result<bool> {
    Ok(Command::new("rerun")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .into_diagnostic()?
        .wait_with_output()
        .await
        .into_diagnostic()?
        .status
        .success())
}

/// Spawn a rerun viewer in the background.
fn spawn_rerun_viewer() -> Result<()> {
    Command::new("rerun")
        .kill_on_drop(false) // Don't kill the rerun process when sindri exits
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .into_diagnostic()?;
    Ok(())
}
