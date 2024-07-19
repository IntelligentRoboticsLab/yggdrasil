use bevy::{app::PluginGroupBuilder, prelude::*};

pub mod audio;
pub mod config;
pub mod control;
pub mod debug;

pub struct CorePlugins;

impl PluginGroup for CorePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(config::ConfigPlugin)
            .add(debug::DebugPlugin)
            .add(audio::AudioPlugin)
    }
}
