use tyr::prelude::*;

mod behaviours;
mod engine;
mod transitions;

use engine::executor;
use transitions::transitions;

pub use engine::{BehaviourEngine, BehaviourType, BehaviourContext};

pub struct BehaviourEngineModule;

impl Module for BehaviourEngineModule {
    fn initialize(self, app: App) -> miette::Result<App> {
        // Is this the correct behaviour to start with?
        Ok(app
            .add_resource(Resource::new(BehaviourEngine::new()))?
            .add_system(executor)
            .add_system(transitions))
    }
}
