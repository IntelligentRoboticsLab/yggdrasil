pub mod engine;

use tyr::prelude::*;

use self::engine::WalkingEngine;

pub struct WalkingEngineModule;

impl Module for WalkingEngineModule {
    fn initialize(self, app: App) -> color_eyre::Result<App> {
        Ok(app
            .add_resource(Resource::new(WalkingEngine::default()))?
            .add_system(engine::walking_engine)
            .add_system(engine::toggle_walking_engine.before(engine::walking_engine)))
    }
}
