pub mod behavior_config;
pub mod behaviors;
pub mod engine;
pub mod primary_state;
pub mod roles;

use crate::prelude::*;

use engine::BehaviorEngineModule;

pub use behavior_config::BehaviorConfig;
#[doc(inline)]
pub use engine::Engine;

/// A collection of modules related to behaviors.
///
/// This module adds the following modules to the application:
/// - [`BehaviorEngineModule`]
pub struct BehaviorModule;

impl Module for BehaviorModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(BehaviorEngineModule)?
            .add_module(primary_state::PrimaryStateModule)
    }
}
