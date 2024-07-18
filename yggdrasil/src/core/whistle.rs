use crate::prelude::*;

#[derive(Default)]
pub struct WhistleState {
    pub detected: bool,
}

pub struct WhistleStateModule;

impl Module for WhistleStateModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_resource(Resource::new(WhistleState::default()))
    }
}
