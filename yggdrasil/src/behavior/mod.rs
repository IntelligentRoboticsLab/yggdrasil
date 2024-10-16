pub mod behavior_config;
pub mod behaviors;
pub mod engine;
pub mod primary_state;
pub mod roles;

use bevy::{app::PluginGroupBuilder, prelude::*};
use engine::BehaviorEnginePlugin;

pub use behavior_config::BehaviorConfig;
#[doc(inline)]
pub use engine::BehaviorEngine;

/// A collection of plugins related to behaviors.
pub struct BehaviorPlugins;

impl PluginGroup for BehaviorPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(BehaviorEnginePlugin)
            .add(primary_state::PrimaryStatePlugin)
    }
}
