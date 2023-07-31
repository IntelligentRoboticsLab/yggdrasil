use clap::Parser;

use crate::config::Config;
use miette::Result;

use super::deploy::{ConfigOptsDeploy, Deploy};

#[derive(Clone, Debug, Parser)]
pub struct ConfigOptsTest {
    /// Robot number
    #[clap(index = 1, name = "robot number")]
    number: u8,

    /// Scan for wired (true) or wireless (false) robots [default: false]
    #[clap(long)]
    lan: bool,

    /// Team number [default: Set in `sindri.toml`]
    #[clap(long)]
    team_number: Option<u8>,
}

#[derive(Parser)]
pub struct Test {
    #[clap(flatten)]
    pub test: ConfigOptsTest,
}

impl Test {
    pub async fn test(self, config: Config) -> Result<()> {
        Deploy {
            deploy: ConfigOptsDeploy::new(
                self.test.number,
                self.test.lan,
                self.test.team_number,
                true,
            ),
        }
        .deploy(config)
        .await
    }
}
