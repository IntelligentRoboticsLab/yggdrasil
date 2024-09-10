use bevy::app::{PluginGroup, PluginGroupBuilder};
mod cycle;
pub use cycle::*;

mod battery_led;
pub mod center_of_mass;
mod lola;
pub mod manager;
mod robot_info;
mod schedule;

const DEFAULT_STIFFNESS: f32 = 0.8;

/// Plugin group which contains convenience plugins for the robot.
pub(super) struct NaoPlugins;

impl PluginGroup for NaoPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(lola::LolaPlugin)
            .add(cycle::CycleTimePlugin)
            .add(battery_led::BatteryLedPlugin)
            .add(manager::NaoManagerPlugin)
            .add(center_of_mass::CenterOfMassPlugin)
    }
}
