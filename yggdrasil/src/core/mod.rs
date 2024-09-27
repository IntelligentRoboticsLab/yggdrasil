use bevy::{app::PluginGroupBuilder, prelude::*};

pub mod audio;
pub mod config;
pub mod debug;
// pub mod whistle;

pub struct CorePlugins;

impl PluginGroup for CorePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(config::ConfigPlugin)
            // .add(whistle::WhistleStatePlugin)
            .add(debug::DebugPlugin)
            .add_group(audio::AudioPlugins)
    }
}
