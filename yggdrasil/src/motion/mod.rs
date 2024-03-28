pub mod arbiter;

use crate::prelude::*;

pub struct MotionModule;

impl Module for MotionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(arbiter::MotionArbiterModule)
    }
}
