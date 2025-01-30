use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

pub mod keyframe;
pub mod odometry;
pub mod path_finding;
mod sensor_data;
pub mod step_planner;
pub mod walk;
pub mod walkv4;

/// Plugin group containing all plugins related to robot motion.
pub struct MotionPlugins;

impl PluginGroup for MotionPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(odometry::OdometryPlugin)
            .add(keyframe::KeyframePlugin)
            .add(step_planner::StepPlannerPlugin)
            .add(sensor_data::VisualizeSensorDataPlugin)
            .add(walk::WalkingEnginePlugin)
            .add(walkv4::Walkv4EnginePlugin)
    }
}
