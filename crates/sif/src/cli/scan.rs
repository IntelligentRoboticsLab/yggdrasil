use clap::Parser;
use futures::future::join_all;
use miette::Result;
use surge_ping::{Client, Config, IcmpPacket, PingIdentifier, PingSequence};
use std::net::IpAddr;
use rand::random;
use std::time::Duration;
use tokio::time;
use colored::Colorize;
use toml;
use serde::Deserialize;
use std::fs;

#[derive(Clone, Debug, Default, Parser)]
pub struct ConfigOptsScan {
    #[clap(long, default_value = "10.0.8")]
    subnet: String,

    #[clap(long, default_value_t = 20)]
    lower: u8,

    #[clap(long, default_value_t = 26)]
    upper: u8,
}

#[derive(Parser)]
#[clap(name = "scan")]
pub struct Scan {
    #[clap(flatten)]
    pub scan: ConfigOptsScan,
}

impl Scan {
    pub async fn scan(self) -> Result<()> {

        println!("Looking for robots...");
        let client_v4 = Client::new(&Config::default()).unwrap();
        let mut tasks = Vec::new();

        for nr in self.scan.lower..=self.scan.upper {
            let ip_address = format!("{}.{}", self.scan.subnet, nr);
            match ip_address.parse() {
                Ok(IpAddr::V4(ip_address)) => {
                    tasks.push(tokio::spawn(ping(client_v4.clone(), IpAddr::V4(ip_address))))
                }
                Ok(IpAddr::V6(ip_address)) => {
                    println!("{} is IPv6. Please enter IPv4.", ip_address);
                }
                Err(err) => println!("{} parse to IP address error: {}", ip_address, err),
            }
        }

        join_all(tasks).await;
        Ok(())
    }
}

async fn ping(client: Client, addr: IpAddr) {
    let payload = [0; 56];
    let mut pinger = client.pinger(addr, PingIdentifier(random())).await;
    pinger.timeout(Duration::from_secs(1));
    let mut interval = time::interval(Duration::from_secs(1));

    for idx in 0..1 {
        interval.tick().await;
        match pinger.ping(PingSequence(idx), &payload).await {
            Ok((IcmpPacket::V4(_), _)) => println!("- {}: {}", pinger.host, "ONLINE".green().bold()),
            Ok((IcmpPacket::V6(_), _)) => println!("{} is IPv6. Please enter IPv4.", pinger.host),
            Err(_) => println!("- {}: {}", pinger.host, "OFFLINE".red().bold()),
        };
    }
}