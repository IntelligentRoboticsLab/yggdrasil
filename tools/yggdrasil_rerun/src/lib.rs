use build_utils::version::Version;

pub mod app;
pub mod cli;
pub mod connection;
pub mod game_controller_view;
pub mod yggdrasil_rerun_view;
pub mod resource;
pub mod state;
pub mod ui;

pub struct RerunControl;

impl Version for RerunControl {
    const BIN_NAME: &'static str = "yggdrasil_rerun";

    const CRATE_PATH: &'static str = "tools/yggdrasil_rerun";

    const PKG_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    const COMMIT_SHORT_HASH: Option<&'static str> = option_env!("YGGDRASIL_RERUN_COMMIT_SHORT_HASH");
    const COMMIT_HASH: Option<&'static str> = option_env!("YGGDRASIL_RERUN_COMMIT_HASH");
    const COMMIT_DATE: Option<&'static str> = option_env!("YGGDRASIL_RERUN_COMMIT_DATE");
}
