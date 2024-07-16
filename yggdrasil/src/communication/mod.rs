pub mod team;

use crate::prelude::*;

use team::TeamCommunicationModule;

/// A collection of modules related to communication.
///
/// This module adds the following modules to the application:
/// - [`CommunicationModule`]
pub struct CommunicationModule;

impl Module for CommunicationModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(TeamCommunicationModule)
    }
}
