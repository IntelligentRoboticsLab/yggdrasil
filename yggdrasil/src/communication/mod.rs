mod team;

use bevy::{app::PluginGroupBuilder, prelude::*};

/// A collection of plugins related to communication.
pub struct CommunicationPlugins;

impl PluginGroup for CommunicationPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(team::TeamCommunicationPlugin)
    }
}
