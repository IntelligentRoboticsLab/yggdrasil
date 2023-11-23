use tyr::prelude::*;

mod behaviours;
mod engine;
mod transitions;

use engine::step;

pub use engine::{BehaviourContext, BehaviourEngine, BehaviourState, ImplBehaviour};

pub struct BehaviourEngineModule;

impl Module for BehaviourEngineModule {
    fn initialize(self, app: App) -> miette::Result<App> {
        Ok(app
            .add_resource(Resource::new(BehaviourEngine::default()))?
            .add_system(step))
    }
}
