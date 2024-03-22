use clap::Parser;
use miette::{IntoDiagnostic, Result};

#[derive(Parser, Debug)]
/// Compile, deploy and run the specified binary to the robot.
pub struct Update {}

impl Update {
    pub async fn update(self) -> Result<()> {
        tokio::process::Command::new("cargo")
            .args(["install", "--locked", "--path", "crates/sindri"])
            .status()
            .await
            .into_diagnostic()?;

        Ok(())
    }
}
