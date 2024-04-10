use std::process::Stdio;

use clap::Parser;
use colored::Colorize;
use miette::{miette, IntoDiagnostic, Result};
use tokio::{process::Command, task::JoinSet};

use crate::config::{Robot, SindriConfig};

/// Scan the current network for online robots.
#[derive(Clone, Debug, Default, Parser)]
pub struct ConfigOptsScan {
    /// The range of robot numbers to be pinged [default: 20 26]
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

        let mut scan_set = JoinSet::new();
        for robot_number in range {
            let robot = config
                .robot(robot_number, self.scan.wired)
                .unwrap_or(Robot::new(
                    "unknown",
                    robot_number,
                    config.team_number,
                    self.scan.wired,
                ));

            scan_set.spawn(ping(robot));
        }

        // wait until all ping commands have been completed
        while let Some(res) = scan_set.join_next().await.transpose().into_diagnostic()? {
            // if something went wrong, we'll want to print the diagnostic!
            if let Err(diagnostic) = res {
                eprintln!("{diagnostic}");
            }
        }
        Ok(())
    }
}

async fn ping(robot: Robot) -> Result<()> {
    let addr = robot.ip();

    let ping_status = Command::new("ping")
        .arg("-W1") // 1 second time out
        .arg("-q") // quiet output
        .arg("-c2") // require only 2 replies
        .arg("-s0") // number of data bytes to be sent
        .arg(addr.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .into_diagnostic()?;

    let online_status = match ping_status.success() {
        true => "ONLINE ".green().bold(),
        false => "OFFLINE".red().bold(),
    };

    println!(
        "[+] {} | {} | {}",
        addr,
        online_status,
        robot.name.white().bold(),
    );

    Ok(())
}
