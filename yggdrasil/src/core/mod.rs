use bevy::{app::PluginGroupBuilder, prelude::*};

pub mod audio;
pub mod config;
#[cfg(feature = "re_control")]
pub mod control;
pub mod debug;

pub struct CorePlugins;

impl PluginGroup for CorePlugins {
    fn build(self) -> PluginGroupBuilder {
        let mut group = PluginGroupBuilder::start::<Self>();

        group = group
            .add(config::ConfigPlugin)
            .add(debug::DebugPlugin)
            .add(audio::AudioPlugin);

        #[cfg(feature = "re_control")]
        {
            group = group.add(control::ControlPlugin);
        }

        group
    }
}
