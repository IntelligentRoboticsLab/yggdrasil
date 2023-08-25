use std::path::PathBuf;

use clap::Parser;
use miette::{IntoDiagnostic, Result};
use sif::{
    cli::{Cli, Commands},
    error::Error,
};
use std::fs;

use sif::config::SifConfig;

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
    let toml_str = fs::read_to_string("sif.toml").expect("Failed to read sif.toml file");
    let sif_config: SifConfig = toml::from_str(&toml_str).expect("Failed to deserialize sif.toml");

    let args = Cli::parse();

    assert_valid_bin(args.bin.clone())?;
    let args_bin = args.bin.clone().unwrap();

    match args.action {
        Commands::Build(opts) => opts.build(args_bin).await?,
        Commands::Upload => todo!(),
        Commands::Scan(opts) => opts.scan(sif_config).await?,
    }

    Ok(())
}
