use miette::Result;
use tyr::prelude::*;

mod behaviour_engine;
mod primary_state;
mod roles;

use behaviour_engine::BehaviorEngineModule;
use primary_state::PrimaryStateModule;
use roles::RoleModule;

pub use behaviour_engine::BehaviourEngine;
pub use primary_state::PrimaryState;
pub use roles::Role;

pub struct BehaviourModule;

impl Module for BehaviourModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(PrimaryStateModule)?
            .add_module(RoleModule)?
            .add_module(BehaviorEngineModule)
    }
}
