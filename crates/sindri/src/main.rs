use clap::Parser;
use miette::Result;
use sindri::{
    cli::{config::ConfigCommand, Cli, Commands},
    config::load_config,
};

#[tokio::main]
async fn main() -> Result<()> {
    let config = match load_config() {
        Ok(config) => config,
        Err(_) => {
            println!("Could not find sindri config, running first time setup");
            ConfigCommand::init()?;
            load_config().unwrap()
        }
    };

    let args = Cli::parse();

    match args.action {
        Commands::Deploy(opts) => opts.deploy(config).await?,
        Commands::Run(opts) => opts.run(config).await?,
        Commands::Scan(opts) => opts.scan(config).await?,
        Commands::Config(opts) => opts.config()?,
    }

    Ok(())
}
