use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

pub mod keyframe;
pub mod odometry;
pub mod path_finding;
pub mod step_planner;
pub mod walk;

/// Plugin group containing all plugins related to robot motion.
pub struct MotionPlugins;

impl PluginGroup for MotionPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(odometry::OdometryPlugin)
            .add(keyframe::KeyframePlugin)
            .add(step_planner::StepPlannerPlugin)
            .add(walk::WalkingEnginePlugin)
    }
}
