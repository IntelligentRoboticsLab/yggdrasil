pub mod proposal;

use crate::prelude::*;

pub struct BallDetectionModule;

impl Module for BallDetectionModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(proposal::BallProposalModule)
    }
}
