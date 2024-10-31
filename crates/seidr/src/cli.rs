use std::net::Ipv4Addr;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Cli {
    /// Robot Ipv4 number
    pub robot_ip: Ipv4Addr,

    /// Max allowed memory usage for rerun, absolute (e.g. "16GB") or relative
    /// (e.g. "50%")
    #[clap(short, long)]
    pub max_mem: Option<String>,
}
