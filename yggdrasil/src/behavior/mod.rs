pub mod behaviors;
pub mod engine;
pub mod roles;

use miette::Result;
use tyr::prelude::*;

use engine::BehaviorEngineModule;

pub struct BehaviorModule;

impl Module for BehaviorModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(BehaviorEngineModule)
    }
}
