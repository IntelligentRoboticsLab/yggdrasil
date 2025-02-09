use bevy::app::PluginGroup;

mod sit;
mod stand;
mod starting;
pub mod walk;

pub(super) struct GaitPlugins;

impl PluginGroup for GaitPlugins {
    fn build(self) -> bevy::app::PluginGroupBuilder {
        bevy::app::PluginGroupBuilder::start::<Self>()
            .add(sit::SitGaitPlugin)
            .add(stand::StandGaitPlugin)
            .add(starting::StartingPlugin)
            .add(walk::WalkGaitPlugin)
    }
}
