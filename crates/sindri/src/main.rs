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
    let config = toml::from_str(&config_data)
        .into_diagnostic()
        .wrap_err("Failed to parse config file!")?;

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
