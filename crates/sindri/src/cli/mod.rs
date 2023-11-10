use clap::Parser;

pub mod build;
pub mod deploy;
pub mod scan;
pub mod test;

#[derive(Parser)]
#[clap(name = "sindri", version)]
pub struct Cli {
    #[clap(subcommand)]
    pub action: Commands,

    /// Enable verbose logging
    #[clap(short)]
    pub v: bool,

    /// Specify bin target
    #[clap(global = true, long, default_value = "yggdrasil")]
    pub bin: String,
}

#[derive(Parser)]
pub enum Commands {
    Build(build::Build),
    Deploy(deploy::Deploy),
    Test(test::Test),
    Scan(scan::Scan),
}
