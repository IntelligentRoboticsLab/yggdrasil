use clap::Parser;

use crate::config::Config;
use miette::Result;

use super::deploy::{ConfigOptsDeploy, Deploy};

#[derive(Parser)]
pub struct Test {
    #[clap(flatten)]
    pub test: ConfigOptsDeploy,
}

impl Test {
    pub async fn test(self, config: Config) -> Result<()> {
        Deploy { deploy: self.test }.deploy(config).await
    }
}
