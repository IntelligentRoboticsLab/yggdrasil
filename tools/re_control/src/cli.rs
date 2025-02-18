use std::net::Ipv4Addr;

use clap::Parser;

use crate::RerunControl;
use build_utils::version::Version;

#[derive(Parser, Debug)]
#[clap(name = "re_control", version = RerunControl::current())]
pub struct Cli {
    /// Robot ip address
    pub robot_ip: Option<Ipv4Addr>,

    /// Max allowed memory usage for rerun, absolute (e.g. "16GB") or relative
    /// (e.g. "50%")
    #[clap(short, long)]
    pub max_mem: Option<String>,
}
