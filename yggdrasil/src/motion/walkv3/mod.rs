use engine::{init_walking_engine, WalkingEnginev3};
use feet::Feet;
use miette::Result;

use crate::prelude::*;

mod action;
mod engine;
mod feet;
mod step;
mod step_state;

pub struct WalkingEngineV3Module;

impl Module for WalkingEngineV3Module {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .init_resource::<Feet>()?
            .add_startup_system(init_walking_engine)?
            .add_staged_system(SystemStage::Finalize, engine::run_walking_enginev3))
    }
}
