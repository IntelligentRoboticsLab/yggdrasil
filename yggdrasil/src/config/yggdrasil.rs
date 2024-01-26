use odal::Config;
use serde::{Deserialize, Serialize};
use tyr::prelude::*;

#[derive(Debug, Deserialize, Serialize)]
pub struct TyrModule {
    pub tasks: TaskModule,
}

impl Config for TyrModule {
    const PATH: &'static str = "tyr.toml";
}

impl Module for TyrModule {
    fn initialize(self, app: App) -> miette::Result<App> {
        app.add_module(self.tasks)
    }
}
