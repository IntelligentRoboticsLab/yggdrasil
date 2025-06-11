use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

pub mod energy_optimizer;
pub mod keyframe;
pub mod obstacle_avoidance;
pub mod path_finding;
pub mod step_planner;
pub mod walking_engine;

/// Plugin group containing all plugins related to robot motion.
pub struct MotionPlugins;

impl PluginGroup for MotionPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(keyframe::KeyframePlugin)
            .add(obstacle_avoidance::ObstacleAvoidancePlugin)
            .add(step_planner::StepPlannerPlugin)
            .add(energy_optimizer::EnergyOptimizerPlugin)
            .add(walking_engine::WalkingEnginePlugin)
    }
}
