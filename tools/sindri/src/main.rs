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

    sindri::version::check_current_version();

    match args.action {
        Commands::Run(opts) => opts.run(config).await?,
        Commands::Scan(opts) => opts.scan(config).await?,
        Commands::Showtime(opts) => opts.showtime(config).await?,
        Commands::ChangeNetwork(opts) => opts.change_network(config).await?,
        Commands::Config(opts) => opts.config()?,
        Commands::Update(opts) => opts.update().await?,
        Commands::DependencyGraph(opts) => opts.generate().await?,
    }

    Ok(())
}
