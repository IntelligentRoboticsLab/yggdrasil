use tyr::prelude::*;

mod behaviours;
mod engine;
mod transitions;

use engine::{executor, transition_behaviour};

pub use engine::{Behaviour, BehaviourContext, BehaviourEngine};

pub struct BehaviourEngineModule;

impl Module for BehaviourEngineModule {
    fn initialize(self, app: App) -> miette::Result<App> {
        Ok(app
            .add_resource(Resource::new(BehaviourEngine::default()))?
            .add_system(transition_behaviour)
            .add_system(executor.after(transition_behaviour))
)
    }
}
