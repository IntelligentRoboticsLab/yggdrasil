use build_manager::version::Version;

pub mod cli;
pub mod config;
pub mod error;

pub struct Sindri;

impl Version for Sindri {
    const BIN_NAME: &'static str = "sindri";
    const CRATE_PATH: &'static str = "tools/sindri";

    const PKG_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    const COMMIT_SHORT_HASH: Option<&'static str> = option_env!("SINDRI_COMMIT_SHORT_HASH");
    const COMMIT_HASH: Option<&'static str> = option_env!("SINDRI_COMMIT_HASH");
    const COMMIT_DATE: Option<&'static str> = option_env!("SINDRI_COMMIT_DATE");
}
