use clap::Parser;
use miette::Result;

/// Changes the default network the robot connects to.
#[derive(Parser, Debug)]
pub struct ChangeNetwork {
    #[clap()]
    pub network: String,
}

impl ChangeNetwork {
    pub async fn change_network(self) -> Result<()> {
        Ok(())
    }
}
