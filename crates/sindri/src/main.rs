use std::path::{Path, PathBuf};

use clap::Parser;
use colored::Colorize;
use miette::{IntoDiagnostic, Result};
use sindri::{
    cli::{Cli, Commands},
    config::Config,
    error::Error,
};

fn assert_valid_bin(bin: &str) -> Result<()> {
    const ERR_MESSAGE: &str = "The `--bin` flag has to be ran in a Cargo workspace.";

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

            if path.ends_with(bin) {
                return Ok(());
            }
        }
    } else {
        Err(Error::CargoError(ERR_MESSAGE.to_string()))?;
    }

    // If the bin exists but we couldn't find it
    Err(Error::CargoError(
        "The specified bin does not exist.".to_string(),
    ))?
}

const CONFIG_FILE: &str = "sindri.toml";

#[tokio::main]
async fn main() -> Result<()> {
    let config_file = Path::new(CONFIG_FILE);
    let config = if !config_file.exists() {
        println!(
            "{}",
            "No config file found! Using default config...".dimmed()
        );
        Config::default()
    } else {
        let config_data = std::fs::read_to_string(config_file).into_diagnostic()?;
        toml::from_str(&config_data).into_diagnostic()?
    };

    let args = Cli::parse();

    assert_valid_bin(&args.bin)?;

    match args.action {
        Commands::Build(opts) => opts.build(&args.bin).await?,
        Commands::Deploy(opts) => opts.deploy(config).await?,
        Commands::Test(opts) => opts.test(config).await?,
        Commands::Scan(opts) => opts.scan(config).await?,
    }

    Ok(())
}
