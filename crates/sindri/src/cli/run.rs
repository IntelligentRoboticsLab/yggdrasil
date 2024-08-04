use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddrV4};
use std::{os::unix::process::CommandExt, process::Stdio};
use yggdrasil::{core::config::showtime::ShowtimeConfig, prelude::Config as OdalConfigTrait};

use clap::Parser;
use colored::Colorize;
use indicatif::ProgressBar;
use miette::{bail, miette, Context, IntoDiagnostic, Result};
use tokio::process::Command;

use crate::{
    cli::robot_ops::{self, ConfigOptsRobotOps, RobotEntry},
    config::SindriConfig,
};

const LOCAL_ROBOT_ID: u8 = 0;
const DEFAULT_PLAYER_NUMBER: u8 = 3;
const DEFAULT_TEAM_NUMBER: u8 = 8;

const DEFAULT_TRACY_PORT: u16 = 8086;

// TODO: refactor config for run
#[derive(Parser, Debug)]
/// Compile, deploy and run the specified binary to the robot.
pub struct Run {
    #[clap(flatten)]
    pub robot_ops: ConfigOptsRobotOps,
    /// Also print debug logs to stdout [default: false]
    #[clap(long, short)]
    pub debug: bool,
}

impl Run {
    /// Compiles, upload and then runs yggdrasil on one robot.
    /// This is done interactively so logs can be seen in the terminal.
    pub async fn run(self, config: SindriConfig) -> Result<()> {
        if self.robot_ops.robots.len() != 1 {
            bail!("Exactly one robot should be specified for the run command")
        }

        // Generate showtime config
        let mut robot_assignments = HashMap::new();
        let RobotEntry {
            robot_number,
            player_number,
        } = self.robot_ops.robots[0];
        if self.robot_ops.local {
            robot_assignments.insert(LOCAL_ROBOT_ID.to_string(), DEFAULT_PLAYER_NUMBER);
        } else if let Some(player_number) = player_number {
            robot_assignments.insert(robot_number.to_string(), player_number);
        } else {
            robot_assignments.insert(robot_number.to_string(), DEFAULT_PLAYER_NUMBER);
        }
        let showtime_config = ShowtimeConfig {
            team_number: self.robot_ops.team.unwrap_or(DEFAULT_TEAM_NUMBER),
            robot_numbers_map: robot_assignments,
        };
        showtime_config
            .store("./deploy/config/generated/showtime.toml")
            .map_err(|e| {
                miette!(format!(
                    "{e} Make sure you run Yggdrasil from the root of the project"
                ))
            })?;

        let local = self.robot_ops.local;
        let rerun = self.robot_ops.rerun;
        let has_rerun = has_rerun().await;

        if rerun && !has_rerun {
            println!(
                "{}: {}",
                "warning".bold().yellow(),
                "rerun is not installed, install it using `cargo install rerun-cli`".white()
            );
        }

        let tracy = self.robot_ops.timings;
        let has_tracy = has_tracy().await;
        if tracy && !has_tracy {
            println!(
                "{}: {}",
                "warning".bold().yellow(),
                "tracy is not installed, install it using your package manager!".white()
            );
        }

        let compile_bar = ProgressBar::new(1);
        let output = robot_ops::Output::Single(compile_bar.clone());
        robot_ops::compile(self.robot_ops.clone(), output.clone()).await?;
        compile_bar.finish_and_clear();

        let robot = self.robot_ops.get_first_robot(&config)?;

        if !self.robot_ops.local {
            output.spinner();
            robot_ops::stop_single_yggdrasil_service(&robot, output.clone()).await?;
            robot_ops::upload_to_robot(&robot.ip(), output.clone()).await?;

            if let Some(network) = self.robot_ops.network {
                output.spinner();
                robot_ops::change_single_network(&robot, network, output.clone()).await?;
            }

            output.finished_deploying(&robot.ip());
        }

        let volume_string = self.robot_ops.volume.to_string();
        let mut envs = vec![("YGGDRASIL_VOLUME", volume_string.as_str())];

        if self.debug {
            envs.push(("RUST_LOG", "debug"));
        }

        if rerun {
            // Always set the host, so that rerun can connect to the correct host.
            // even if the host doesn't have rerun viewer installed, there could be
            // some case where the viewer is launched through a different method than the cli.
            let rerun_host: Result<_> = std::env::var("RERUN_HOST").or_else(|_| {
                let mut local_ip = local_ip_address::local_ip().into_diagnostic()?;

                // Make sure the wired bit is set if we're running with `--wired`.
                if self.robot_ops.wired {
                    if let IpAddr::V4(ipv4) = &mut local_ip {
                        *ipv4 |= Ipv4Addr::new(0, 1, 0, 0);
                    }
                }

                Ok(local_ip.to_string())
            });

            envs.push(("RERUN_HOST", rerun_host?.leak()));

            if has_rerun {
                spawn_rerun_viewer()?;
            }
        }

        if tracy && has_tracy {
            // Default to the robot's IP address, but allow the user to override it.
            let robot_ip = if local {
                Ipv4Addr::LOCALHOST
            } else {
                robot.ip()
            };

            let tracy_client_ip = std::env::var("TRACY_CLIENT")
                .unwrap_or_else(|_| format!("{robot_ip}:{DEFAULT_TRACY_PORT}",))
                .parse()
                .into_diagnostic()
                .wrap_err("Invalid tracy client ip address!")?;

            spawn_tracy_profiler(tracy_client_ip)?;
        }

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

/// Check if the `tracy` binary is installed.
///
/// We check if the `tracy` binary is installed by running `tracy --help` and checking if the
/// command was successful.
async fn has_tracy() -> bool {
    async fn get_tracy_version() -> Result<bool> {
        // Tracy version is listed in the help output
        Ok(Command::new("tracy")
            .arg("--help")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .into_diagnostic()?
            .success())
    }

    get_tracy_version().await.is_ok_and(|success| success)
}

/// Spawn the Tracy profiler in the background, connecting to the given address.
fn spawn_tracy_profiler(address: SocketAddrV4) -> Result<()> {
    let mut process = std::process::Command::new("tracy");

    process
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);

    Command::from(process)
        .arg("-a")
        .arg(address.ip().to_string())
        .arg("-p")
        .arg(address.port().to_string())
        .kill_on_drop(false)
        .spawn()
        .into_diagnostic()?;

    Ok(())
}
