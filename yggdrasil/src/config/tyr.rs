use crate::prelude::*;

use odal::Config;
use serde::{Deserialize, Serialize};
use tyr::tasks::{TaskConfig, TaskModule};

use super::ConfigResource;

pub struct TyrModule;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TyrConfig {
    tasks: TaskConfig,
}

impl Config for TyrConfig {
    const PATH: &'static str = "tyr.toml";
}

impl Module for TyrModule {
    fn initialize(self, app: App) -> miette::Result<App> {
        app.init_config::<TyrConfig>()?.add_module(TaskModule)
    }
}
