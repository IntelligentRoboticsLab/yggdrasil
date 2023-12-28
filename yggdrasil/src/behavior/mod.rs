pub mod behaviors;
pub mod engine;
pub mod roles;

use miette::Result;
use tyr::prelude::*;

use engine::BehaviorEngineModule;

#[doc(inline)]
pub use engine::Engine;

/// A collection of modules related to behaviors.
///
/// This module adds the following modules to the application:
/// - [`BehaviorEngineModule`]
pub struct BehaviorModule;

impl Module for BehaviorModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_module(BehaviorEngineModule)
    }
}
