use clap::Parser;
use colored::Colorize;
use futures::future::join_all;
use miette::{miette, Result};
use rand::random;
use std::net::IpAddr;
use std::time::Duration;
use surge_ping::{Client, Config, IcmpPacket, PingIdentifier, PingSequence};
use tokio::time;

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

    /// Team number [default: 8]
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
        let client_v4 = Client::new(&Config::default()).unwrap();
        let mut tasks = Vec::new();

        for nr in self.scan.lower..=self.scan.upper {
            tasks.push(tokio::spawn(ping(
                client_v4.clone(),
                nr,
                sif_config.clone(),
                self.scan.clone(),
            )));
        }
        join_all(tasks).await;
        Ok(())
    }
}

async fn ping(
    client: Client,
    robot_number: u8,
    sif_config: SifConfig,
    opts: ConfigOptsScan,
) -> Result<()> {
    let addr = sif_config
        .robots
        .get(&robot_number)
        .ok_or_else(|| miette!("Robot {} not found", robot_number))?
        .get_ip(opts.team_number.unwrap_or(sif_config.team_number), opts.lan);
    let payload = [0; 8];
    let mut pinger = client
        .pinger(IpAddr::V4(addr), PingIdentifier(random()))
        .await;
    pinger.timeout(Duration::from_secs(1));
    let mut interval = time::interval(Duration::from_secs(1));

    for idx in 0..1 {
        interval.tick().await;
        match pinger.ping(PingSequence(idx), &payload).await {
            Ok((IcmpPacket::V4(_), _)) => println!(
                "[+] {} => {}: {}",
                pinger.host,
                sif_config.get_robot_name(robot_number).white().bold(),
                "ONLINE".green().bold()
            ),
            Ok((IcmpPacket::V6(_), _)) => {}
            Err(_) => println!(
                "[+] {} | {} | {}",
                "OFFLINE".red().bold(),
                pinger.host,
                sif_config.get_robot_name(robot_number).white().bold(),
            ),
        };
    }
    Ok(())
}
