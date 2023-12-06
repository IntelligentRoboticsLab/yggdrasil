use tyr::prelude::*;

mod behaviours;
mod engine;

use engine::step;

pub use engine::{Behave, BehaviourEngine, Context};

pub struct BehaviorEngineModule;

impl Module for BehaviorEngineModule {
    fn initialize(self, app: App) -> miette::Result<App> {
        Ok(app.init_resource::<BehaviourEngine>()?.add_system(step))
    }
}
