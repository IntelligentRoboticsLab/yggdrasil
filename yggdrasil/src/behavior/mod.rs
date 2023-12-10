//Make pub behaviours and Roles
pub mod behaviors;
mod engine;
pub mod roles;

pub use engine::{Behavior, Context, Engine};

use miette::Result;
use tyr::prelude::*;

use engine::BehaviorEngineModule;

pub struct BehaviorModule;

impl Module for BehaviorModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(BehaviorEngineModule)
    }
}
