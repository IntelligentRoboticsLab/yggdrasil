use std::net::Ipv4Addr;

use clap::Parser;

#[derive(Parser)]
pub struct Cli {
    /// Robot Ipv4 number
    pub robot_ip: Ipv4Addr,
    /// Max allowed memory usage for rerun in MB
    #[clap(short, long)]
    pub max_mem: Option<u64>,
}
