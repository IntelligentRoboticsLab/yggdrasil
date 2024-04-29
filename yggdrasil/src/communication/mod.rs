pub mod team;

use crate::prelude::*;

use team::TeamCommunicationModule;

/// A collection of modules related to behaviors.
///
/// This module adds the following modules to the application:
/// - [`BehaviorEngineModule`]
pub struct CommunicationModule;

impl Module for CommunicationModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(TeamCommunicationModule)
    }
}
