use clap::Parser;
use miette::{miette, IntoDiagnostic, Result};

#[derive(Parser, Debug)]
/// Compile, deploy and run the specified binary to the robot.
pub struct Update {}

impl Update {
    pub async fn update(self) -> Result<()> {
        crate::cargo::assert_valid_bin("sindri")
            .map_err(|_| miette!("Command must be executed from the yggdrasil directory"))?;

        tokio::process::Command::new("cargo")
            .args(["install", "--locked", "--path", "crates/sindri"])
            .status()
            .await
            .into_diagnostic()?;

        Ok(())
    }
}
