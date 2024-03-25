use clap::Parser;
use miette::{miette, IntoDiagnostic, Result};

/// Update the current `sindri` installation.
#[derive(Parser, Debug)]
pub struct Update;

impl Update {
    pub async fn update(self) -> Result<()> {
        crate::cargo::find_bin_manifest("sindri")
            .map_err(|_| miette!("Command must be executed from the yggdrasil directory"))?;

        tokio::process::Command::new("cargo")
            .args(["install", "--locked", "--path", "crates/sindri"])
            .status()
            .await
            .into_diagnostic()?;

        Ok(())
    }
}
