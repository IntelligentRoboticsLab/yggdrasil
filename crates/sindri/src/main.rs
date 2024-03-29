use std::path::Path;

use clap::Parser;
use miette::{Context, IntoDiagnostic, Result};
use sindri::{
    cargo::assert_valid_bin,
    cli::{Cli, Commands},
};

const CONFIG_FILE: &str = "sindri.toml";

#[tokio::main]
async fn main() -> Result<()> {
    let config_file = Path::new(CONFIG_FILE);

    let config_data = std::fs::read_to_string(config_file)
        .into_diagnostic()
        .wrap_err("Failed to read config file! Does it exist?")?;
    let config = toml::de::from_str(&config_data)
        .into_diagnostic()
        .wrap_err("Failed to parse config file!")?;

    let args = Cli::parse();

    assert_valid_bin(&args.bin)?;

    match args.action {
        Commands::Deploy(opts) => opts.deploy(config).await?,
        Commands::Run(opts) => opts.run(config).await?,
        Commands::Scan(opts) => opts.scan(config).await?,
        Commands::Pregame(opts) => opts.pregame(config).await?,
    }

    Ok(())
}
