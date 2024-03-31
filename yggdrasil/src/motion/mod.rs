use crate::prelude::*;

pub struct MotionModule;

impl Module for MotionModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app)
    }
}
