use std::path::PathBuf;

use clap::Parser;
use miette::{IntoDiagnostic, Result};
use sindri::{
    cli::{Cli, Commands},
    error::Error,
    config::SindriConfig,
};
use std::fs;

fn assert_valid_bin(bin: Option<String>) -> Result<()> {
    const ERR_MESSAGE: &str = "The `--bin` flag has to be ran in a Cargo workspace.";

    if let Some(ref bin) = bin {
        let manifest =
            cargo_toml::Manifest::from_path("./Cargo.toml").map_err(Error::CargoManifestError)?;
        if let Some(workspace) = manifest.workspace {
            for item in workspace.members.iter() {
                let path = PathBuf::from(item);

                if !path.exists() {
                    continue;
                }

                if !path.is_dir() {
                    continue;
                }

                if path.ends_with(bin.clone()) {
                    return Ok(());
                }
            }
        } else {
            return Err(Error::CargoError(ERR_MESSAGE.to_string()))?;
        }
    }

    // If the bin exists but we couldn't find it
    if bin.is_some() {
        return Err(Error::CargoError(
            "The specified bin does not exist.".to_string(),
        ))?;
    }

    Err(Error::CargoError("Invalid bin specified".to_string())).into_diagnostic()?
}

#[tokio::main]
async fn main() -> Result<()> {
    let toml_str = fs::read_to_string("sindri.toml").into_diagnostic()?;
    let sindri_config: SindriConfig = toml::from_str(&toml_str).into_diagnostic()?;

    let args = Cli::parse();

    assert_valid_bin(args.bin.clone())?;
    let args_bin = args.bin.clone().unwrap();

    match args.action {
        Commands::Build(opts) => opts.build(args_bin).await?,
        Commands::Deploy(opts) => opts.deploy(sindri_config).await?,
        Commands::Scan(opts) => opts.scan(sindri_config).await?,
    }

    Ok(())
}
