use miette::Result;
use tyr::prelude::*;

mod behaviour_engine;
mod primary_state;
mod roles;

use behaviour_engine::BehaviourEngineModule;
use primary_state::PrimaryStateModule;
use roles::RoleModule;

pub use behaviour_engine::{BehaviourEngine, BehaviourType};
pub use roles::Role;

pub struct BehaviourModule;

impl Module for BehaviourModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_module(PrimaryStateModule)?
            .add_module(RoleModule)?
            .add_module(BehaviourEngineModule)?)
    }
}
