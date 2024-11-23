use std::{
    net::Ipv4Addr,
    process::{ExitStatus, Stdio},
};

use crate::cli::robot_ops::NameOrNum;
use clap::Parser;
use colored::Colorize;
use futures::{stream::FuturesOrdered, TryStreamExt};
use miette::{miette, IntoDiagnostic, Result};
use tokio::process::Command;

use crate::config::{Robot, SindriConfig};

/// Scan the current network for online robots.
#[derive(Clone, Debug, Default, Parser)]
pub struct ConfigOptsScan {
    /// The range of robot numbers to be pinged [defaults to [min, max] robot numbers in the sindri config]
    #[clap(short, long, num_args = 2)]
    range: Option<Vec<u8>>,

    /// Scan for wired (true) or wireless (false) robots [default: false]
    #[clap(short, long)]
    wired: bool,

    /// Team number [default: Set in `sindri.toml`]
    #[clap(short, long)]
    team_number: Option<u8>,
}

#[derive(Parser)]
#[clap(name = "scan")]
pub struct Scan {
    #[clap(flatten)]
    pub scan: ConfigOptsScan,
}

impl Scan {
    /// Scan a range of ips to check if the robots are online
    pub async fn scan(self, config: SindriConfig) -> Result<()> {
        let range = self
            .scan
            .range
            .map_or(config.robot_range()?, |r| r[0]..=r[1]);

        if range.is_empty() {
            return Err(miette!(
                "Invalid range format! The range should be in the following format: [lower upper]"
            ));
        }

        println!("Looking for robots...");

        let robots = range
            .map(|robot_number| match self.scan.team_number {
                Some(team_number) => {
                    Robot::new("unknown", robot_number, team_number, self.scan.wired)
                }
                None => config
                    .robot(&NameOrNum::Number(robot_number), self.scan.wired)
                    .unwrap_or_else(|| {
                        Robot::new("unknown", robot_number, config.team_number, self.scan.wired)
                    }),
            })
            .collect::<Vec<_>>();

        let scans = robots
            .iter()
            .map(|robot| ping(robot.ip()))
            .collect::<FuturesOrdered<_>>()
            .try_collect::<Vec<_>>()
            .await?;

        for (robot, status) in robots.into_iter().zip(scans) {
            print_ping_status(robot, status.success());
        }

        Ok(())
    }
}

async fn ping(ip: Ipv4Addr) -> Result<ExitStatus> {
    let ping_status = Command::new("ping")
        .arg("-W1") // 1 second time out
        .arg("-q") // quiet output
        .arg("-c2") // require only 2 replies
        .arg("-s0") // number of data bytes to be sent
        .arg(ip.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .into_diagnostic()?;

    Ok(ping_status)
}

fn print_ping_status(robot: Robot, online: bool) {
    let online_status = if online {
        "ONLINE ".green().bold()
    } else {
        "OFFLINE".red().bold()
    };

    println!(
        "[+] {} | {} | {}",
        robot.ip(),
        online_status,
        robot.name.white().bold(),
    );
}
