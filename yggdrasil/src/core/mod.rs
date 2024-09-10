use bevy::{app::PluginGroupBuilder, prelude::*};

#[cfg(feature = "alsa")]
pub mod audio;
pub mod config;
pub mod debug;
pub mod ml;
pub mod whistle;

pub struct CorePlugins;

impl PluginGroup for CorePlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(config::ConfigPlugin)
            .add(ml::MlPlugin)
            .add(whistle::WhistleStatePlugin)
            .add(debug::DebugPlugin)
            .add(audio::AudioPlugin)
    }
}
