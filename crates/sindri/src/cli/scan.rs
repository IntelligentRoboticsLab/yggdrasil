use std::process::Stdio;

use clap::Parser;
use colored::Colorize;
use miette::{miette, IntoDiagnostic, Result};
use tokio::{process::Command, task::JoinSet};

use crate::config::SindriConfig;

/// Configuration options for the scanning system, specifying the IP addresses to be pinged.
#[derive(Clone, Debug, Default, Parser)]
pub struct ConfigOptsScan {
    /// The range of robot numbers to be pinged [default: 20 26]
    #[clap(long, num_args = 2, default_values_t = [20, 26])]
    range: Vec<u8>,

    /// Scan for wired (true) or wireless (false) robots [default: false]
    #[clap(long)]
    lan: bool,

    /// Team number [default: Set in `sindri_config.toml`]
    #[clap(long)]
    team_number: Option<u8>,
}

#[derive(Parser)]
#[clap(name = "scan")]
pub struct Scan {
    #[clap(flatten)]
    pub scan: ConfigOptsScan,
}

impl Scan {
    pub async fn scan(self, sindri_config: SindriConfig) -> Result<()> {
        if self.scan.range[0] > self.scan.range[1] {
            return Err(miette!("The range should be in the form: [lower upper]"));
        }
        println!("Looking for robots...");
        let mut scan_set = JoinSet::new();
        for robot_number in self.scan.range[0]..=self.scan.range[1] {
            scan_set.spawn(ping(robot_number, sindri_config.clone(), self.scan.clone()));
        }

        // wait until all ping commands have been completed
        while let Some(res) = scan_set.join_next().await {
            // if something went wrong, we'll want to print the diagnostic!
            if let Err(diagnostic) = res.into_diagnostic()? {
                eprintln!("{diagnostic}");
            }
        }
        Ok(())
    }
}

async fn ping(robot_number: u8, sindri_config: SindriConfig, opts: ConfigOptsScan) -> Result<()> {
    let addr = format!(
        "10.{}.{}.{}",
        u8::from(opts.lan),
        opts.team_number.unwrap_or(sindri_config.team_number),
        robot_number
    );

    let ping = Command::new("ping")
        .arg("-W1") // 1 second time out
        .arg("-q") // quiet output
        .arg("-c2") // require only 2 replies
        .arg("-s0") // number of data bytes to be sent
        .arg(addr.clone())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .into_diagnostic()?
        .wait()
        .await
        .into_diagnostic()?;

    if ping.success() {
        println!(
            "[+] {} | {} | {}",
            addr,
            "ONLINE ".green().bold(),
            sindri_config.get_robot_name(robot_number).white().bold(),
        );
        return Ok(());
    }

    println!(
        "[+] {} | {} | {}",
        addr,
        "OFFLINE".red().bold(),
        sindri_config.get_robot_name(robot_number).white().bold(),
    );

    Ok(())
}
