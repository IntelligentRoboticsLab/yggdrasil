use clap::Parser;
use colored::Colorize;
use indicatif::{HumanDuration, ProgressBar, ProgressStyle};
use std::{net::Ipv4Addr, process::Stdio, time::Duration};
use miette::{IntoDiagnostic, Result};
use crate::cargo;
use tokio::process::Command;

const CONTROL_BINARY_PATH: &str = "./target/release/control";
const CONTROL_BINARY: &str = "control";

#[derive(Clone, Debug, Parser)]
pub struct RerunArgs {
    /// Whether to embed the rerun viewer for debugging [default: false]
    #[clap(long, short, default_value(None))]
    pub rerun: Option<Option<String>>,

    /// Set a memory limit for the rerun viewer. --rerun required
    #[clap(long, requires = "rerun")]
    pub rerun_mem_limit: Option<String>,
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

/// Compiles the re_control binary
pub async fn build_rerun_control() -> Result<()> {
    let features = vec![];
    let envs = Vec::new();

    let pb = ProgressBar::new_spinner();
    pb.enable_steady_tick(Duration::from_millis(80));
    pb.set_style(
        ProgressStyle::with_template(
            "   {prefix:.green.bold} re_control {msg} {spinner:.green.bold}",
        )
        .unwrap()
        .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "),
    );

    pb.set_prefix(format!("{}", "Compiling"));
    pb.set_message(format!(
        "{} {}{}",
        "(release:".dimmed(),
        "true".red(),
        ")".dimmed(),
    ));

    cargo::build(
        CONTROL_BINARY,
        cargo::Profile::Release,
        None,
        &features,
        Some(envs),
    )
    .await?;

    pb.println(format!(
        "{} {} {} {}{}",
        "   Compiling".green().bold(),
        "re_control".bold(),
        "(release:".dimmed(),
        "true".red(),
        ")".dimmed(),
    ));

    pb.println(format!(
        "{} in {}",
        "    Finished".green().bold(),
        HumanDuration(pb.elapsed()),
    ));
    pb.reset_elapsed();

    Ok(())
}

/// Spawn a rerun viewer in the background.
pub async fn spawn_rerun_viewer(robot_ip: Ipv4Addr, memory_limit: Option<String>) -> Result<()> {
    let mut args = vec![];
    // Set robot ip to connection the viewer with
    args.push(robot_ip.to_string());

    // Additionally set a memory limit for the viewer
    if let Some(memory_limit) = memory_limit {
        args.push("--max-mem".to_string());
        args.push(memory_limit.to_string());
    }

    Command::new(CONTROL_BINARY_PATH)
        .args(args)
        .stdin(Stdio::inherit())
        .stderr(Stdio::inherit())
        .kill_on_drop(false)
        .spawn()
        .into_diagnostic()?;

    Ok(())
}
