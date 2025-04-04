use build_utils::version::Version;
use clap::Parser;
use miette::Result;
use sindri::{
    cli::{config::ConfigCommand, Cli, Commands},
    config::load_config,
    Sindri,
};

#[tokio::main]
async fn main() -> Result<()> {
    let config = if let Ok(config) = load_config() {
        config
    } else {
        println!("Could not find sindri config, running first time setup");
        ConfigCommand::init()?;
        load_config().unwrap()
    };

    let args = Cli::parse();
    Sindri::check_current_version();

    match args.action {
        Commands::Run(opts) => opts.run(config).await?,
        Commands::Scan(opts) => opts.scan(config).await?,
        Commands::Showtime(opts) => opts.showtime(config).await?,
        Commands::ChangeNetwork(opts) => opts.change_network(config).await?,
        Commands::Shutdown(opts) => opts.shutdown(config).await?,
        Commands::Config(opts) => opts.config()?,
        Commands::Update(opts) => opts.update().await?,
    }

    Ok(())
}
