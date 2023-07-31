use clap::Parser;

pub mod build;

#[derive(Parser)]
#[clap(name = "sif")]
pub struct Cli {
    #[clap(subcommand)]
    pub action: Commands,

    /// Enable verbose logging
    #[clap(short)]
    pub v: bool,

    /// Specify bin target
    #[clap(global = true, long, default_value = "yggdrasil")]
    pub bin: Option<String>,
}

#[derive(Parser)]
pub enum Commands {
    Build(build::Build),
    Upload,
}
