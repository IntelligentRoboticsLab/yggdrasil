use crate::config::SindriConfig;
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct ConfigOptsDownload {
    /// Number of the robot to deploy to.
    #[clap(index = 1, name = "robot-number")]
    pub number: u8,

    /// Scan for wired (true) or wireless (false) robots [default: false]
    #[clap(short, long)]
    pub wired: bool,

    /// Team number [default: Set in `sindri.toml`]
    #[clap(short, long)]
    pub team_number: Option<u8>,
}

impl ConfigOptsDownload {
    #[must_use]
    pub fn new(number: u8, wired: bool, team_number: Option<u8>) -> Self {
        Self {
            number,
            wired,
            team_number,
        }
    }
}

/// Compile and deploy the specified binary to the robot.
#[derive(Parser)]
pub struct Download {
    #[clap(flatten)]
    pub deploy: ConfigOptsDownload,
}

impl Download {
    pub async fn download(self, _config: SindriConfig) -> miette::Result<()> {
        Ok(())
    }
}
