//! This module implements pathfinding.

pub mod finding;
pub mod geometry;
pub mod obstacles;
pub mod planner;
pub mod visualization;

pub use finding::Target;
pub use planner::PathPlanner;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::core::config::ConfigExt;
use odal::Config;

use obstacles::{add_static_obstacles, obstacles_changed, update_colliders};
use visualization::{init_visualization, visualize_obstacles, visualize_path};

/// Plugin providing pathfinding capabilities.
pub struct PathPlugin;

impl Plugin for PathPlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<PathConfig>()
            .add_systems(Startup, init_path_planner)
            .add_systems(Startup, add_static_obstacles)
            .add_systems(PostStartup, init_visualization)
            .add_systems(
                Update,
                (update_colliders, visualize_obstacles)
                    .chain()
                    .run_if(obstacles_changed),
            )
            .add_systems(
                Update,
                visualize_path.run_if(resource_changed::<PathPlanner>),
            );
    }
}

fn init_path_planner(mut commands: Commands, config: Res<PathConfig>) {
    commands.insert_resource(PathPlanner::new(*config));
}

/// Struct containing the configuration for the pathfinding.
#[derive(Resource, Serialize, Deserialize, Debug, Copy, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PathConfig {
    /// The radius of the robot's collider in meters.
    pub robot_radius: f32,
    /// The radius of the arc to ease into the path.
    pub ease_in_radius: f32,
    /// The radius of the arc to ease out of the path.
    pub ease_out_radius: f32,
    /// The maximum distance the robot may be from the path.
    pub start_tolerance: f32,
    /// The maximum distance the target may be from the path.
    pub target_tolerance: f32,
    /// The deadband of the perpendicular error.
    pub perpendicular_deadband: f32,
    /// The speed at which the perpendicular error gets corrected.
    pub perpendicular_speed: f32,
    /// The deadband of the angular error.
    pub angular_deadband: f32,
    /// The speed at which the angular error gets corrected while walking.
    pub angular_speed: f32,
    /// The angular threshold at which the robot stops to turn.
    pub stop_and_turn_threshold: f32,
    /// The walking speed of the robot.
    pub walk_speed: f32,
    /// The turning speed of the robot while it's standing still.
    pub turn_speed: f32,
    /// The turning speed of the robot while it's walking.
    pub walking_turn_speed: f32,
}

impl Config for PathConfig {
    const PATH: &'static str = "path_planner.toml";
}
