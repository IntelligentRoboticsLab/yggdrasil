use std::net::Ipv4Addr;

use clap::Parser;

#[derive(Parser, Debug)]
pub struct Cli {
    /// Robot ip address
    pub robot_ip: Option<Ipv4Addr>,

    /// Max allowed memory usage for rerun, absolute (e.g. "16GB") or relative
    /// (e.g. "50%")
    #[clap(short, long)]
    pub max_mem: Option<String>,
}
