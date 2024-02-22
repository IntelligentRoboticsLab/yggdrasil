use crate::prelude::*;

pub struct LineDetectionModule;

impl Module for LineDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app)
    }
}
