use clap::Parser;
use miette::{IntoDiagnostic, Result};
use std::{
    net::{IpAddr, Ipv4Addr},
    process::Stdio,
};
use tokio::process::Command;

#[derive(Clone, Debug, Parser)]
pub struct RerunArgs {
    /// Whether to embed the rerun viewer for debugging [default: false]
    #[clap(long, short, default_value(None))]
    pub rerun: Option<Option<String>>,

    /// Set a memory limit for the rerun viewer. --rerun required
    #[clap(long, requires = "rerun")]
    pub rerun_mem_limit: Option<String>,

    // Whether to pipe the output of rerun to the terminal
    #[clap(long, action, requires = "rerun")]
    pub rerun_log: bool,
}

pub fn setup_rerun_host(wired: bool) -> Result<String> {
    std::env::var("RERUN_HOST").or_else(|_| {
        let mut local_ip = local_ip_address::local_ip().into_diagnostic()?;

        // Make sure the wired bit is set if we're running with `--wired`.
        if wired {
            if let IpAddr::V4(ipv4) = &mut local_ip {
                *ipv4 |= Ipv4Addr::new(0, 1, 0, 0);
            }
        }

        Ok(local_ip.to_string())
    })
}

/// Check if the `rerun` binary is installed.
///
/// We check if the `rerun` binary is installed by running `rerun --version` and checking if the
/// command was successful.
pub async fn has_rerun() -> bool {
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

/// Check if the `rsync` binary is installed.
///
/// We check if the `rsync` binary is installed by running `rsync --version` and checking if the
/// command was successful.
pub async fn has_rsync() -> bool {
    async fn get_rsync_version() -> Result<bool> {
        Ok(Command::new("rsync")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .into_diagnostic()?
            .success())
    }

    get_rsync_version().await.is_ok_and(|success| success)
}

/// Spawn a rerun viewer in the background.
fn spawn_rerun_viewer(
    robot_ip: Ipv4Addr,
    memory_limit: Option<String>,
    rerun_log: bool,
) -> Result<()> {
    let mut args = vec![];
    // Set robot ip to connection the viewer with
    args.push(robot_ip.to_string());

    // Additionally set a memory limit for the viewer
    if let Some(memory_limit) = memory_limit {
        args.push("--max-mem".to_string());
        args.push(memory_limit.to_string());
    }

    let (stdio_out, stdio_err) = {
        if rerun_log {
            (Stdio::inherit(), Stdio::inherit())
        } else {
            (Stdio::null(), Stdio::null())
        }
    };

    Command::new("re_control")
        .args(args)
        .stdin(Stdio::null())
        .stdout(stdio_out)
        .stderr(stdio_err)
        .kill_on_drop(false)
        .spawn()
        .into_diagnostic()?;

    Ok(())
}

pub async fn run_re_control(
    robot_ip: Ipv4Addr,
    memory_limit: Option<String>,
    rerun_log: bool,
) -> Result<()> {
    spawn_rerun_viewer(robot_ip, memory_limit, rerun_log)?;

    Ok(())
}
