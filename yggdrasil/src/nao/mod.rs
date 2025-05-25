use bevy::app::{PluginGroup, PluginGroupBuilder};

mod battery_led;
mod center_of_mass;
mod center_of_pressure;
mod cycle;
mod lola;
mod manager;
mod robot_info;
mod sim_lola;

pub use center_of_mass::*;
pub use center_of_pressure::*;
pub use cycle::*;
pub use manager::*;
pub use robot_info::*;

/// Plugin group which contains convenience plugins for the robot.
pub struct NaoPlugins;

impl PluginGroup for NaoPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(lola::LolaPlugin)
            .add(cycle::CycleTimePlugin)
            .add(battery_led::BatteryLedPlugin)
            .add(manager::NaoManagerPlugin)
            .add(center_of_mass::CenterOfMassPlugin)
            .add(center_of_pressure::CenterOfPressurePlugin)
    }
}

pub struct SimulationNaoPlugins;

impl PluginGroup for SimulationNaoPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(sim_lola::SimLolaPlugin)
            .add(cycle::CycleTimePlugin)
            .add(battery_led::BatteryLedPlugin)
            .add(manager::NaoManagerPlugin)
            .add(center_of_mass::CenterOfMassPlugin)
            .add(center_of_pressure::CenterOfPressurePlugin)
    }
}
