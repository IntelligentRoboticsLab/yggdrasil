use tyr::prelude::*;

mod behaviours;
mod engine;
mod transitions;

use engine::{executor, initializer};
use transitions::transitions;

pub use engine::{BehaviourEngine, BehaviourType};

pub struct BehaviourEngineModule;

impl Module for BehaviourEngineModule {
    fn initialize(self, app: App) -> miette::Result<App> {
        // Is this the correct behaviour to start with?
        Ok(app
            .add_startup_system(initializer)?
            .add_system(executor)
            .add_system(transitions))
    }
}
