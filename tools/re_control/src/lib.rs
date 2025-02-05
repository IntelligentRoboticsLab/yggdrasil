use build_utils::version::Version;

pub mod app;
pub mod cli;
pub mod connection;
pub mod re_control_view;
pub mod resource;
pub mod state;
pub mod ui;

pub struct RerunControl;

impl Version for RerunControl {
    const BIN_NAME: &'static str = "re_control";

    const CRATE_PATH: &'static str = "tools/re_control";

    const PKG_VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
    const COMMIT_SHORT_HASH: Option<&'static str> = option_env!("RE_CONTROL_COMMIT_SHORT_HASH");
    const COMMIT_HASH: Option<&'static str> = option_env!("RE_CONTROL_COMMIT_HASH");
    const COMMIT_DATE: Option<&'static str> = option_env!("RE_CONTROL_COMMIT_DATE");
}
