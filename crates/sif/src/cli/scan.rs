use std::process::Stdio;

use clap::Parser;
use colored::Colorize;
use miette::{IntoDiagnostic, Result};
use tokio::{process::Command, task::JoinSet};

use crate::config::SifConfig;

/// Configuration options for the scanning system, specifying the IP addresses to be pinged.
#[derive(Clone, Debug, Default, Parser)]
pub struct ConfigOptsScan {
    /// The lower bound of the search range for the last octet of the IP address [default: 20]
    #[clap(long, default_value_t = 20)]
    lower: u8,

    /// The upper bound of the search range for the last octet of the IP address [default: 26]
    #[clap(long, default_value_t = 26)]
    upper: u8,

    /// Scan for wired (true) or wireless (false) robots [default: false]
    #[clap(long)]
    lan: bool,

    /// Team number [default: Set in `sif_config.toml`]
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
    pub async fn scan(self, sif_config: SifConfig) -> Result<()> {
        println!("Looking for robots...");
        let mut scan_set = JoinSet::new();
        for robot_number in self.scan.lower..=self.scan.upper {
            scan_set.spawn(ping(robot_number, sif_config.clone(), self.scan.clone()));
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

async fn ping(robot_number: u8, sif_config: SifConfig, opts: ConfigOptsScan) -> Result<()> {
    let addr = format!(
        "10.{}.{}.{}",
        u8::from(opts.lan),
        opts.team_number.unwrap_or(sif_config.team_number),
        robot_number
    );

    let code = Command::new("ping")
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

    if code.success() {
        println!(
            "[+] {} => {}: {}",
            addr,
            sif_config.get_robot_name(robot_number).white().bold(),
            "ONLINE".green().bold()
        );
        return Ok(());
    }

    println!(
        "[+] {} | {} | {}",
        "OFFLINE".red().bold(),
        addr,
        sif_config.get_robot_name(robot_number).white().bold(),
    );

    Ok(())
}
