mod engine;
mod primary_state;
mod roles;

pub use engine::{behaviors, Behavior, Context, Engine};
pub use primary_state::PrimaryState;
pub use roles::Role;

use miette::Result;
use tyr::prelude::*;

use engine::BehaviorEngineModule;
use primary_state::PrimaryStateModule;
use roles::RoleModule;

pub struct BehaviorModule;

impl Module for BehaviorModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(PrimaryStateModule)?
            .add_module(RoleModule)?
            .add_module(BehaviorEngineModule)
    }
}
