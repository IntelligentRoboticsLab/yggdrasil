//! This module implements pathfinding.

pub mod finding;
pub mod geometry;
pub mod obstacles;
pub mod planning;
pub mod visualization;

pub use finding::Target;
pub use planning::PathPlanner;


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
    pub robot_radius: f32,
    pub ease_in: f32,
    pub ease_out: f32,
    pub start_tolerance: f32,
    pub target_tolerance: f32,
    pub perpendicular_tolerance: f32,
    pub min_angular_tolerance: f32,
    pub max_angular_tolerance: f32,
    pub walk_speed: f32,
    pub turn_speed: f32,
    pub walking_turn_speed: f32,
    pub perpendicular_speed: f32,
    pub angular_speed: f32,
}

impl Config for PathConfig {
    const PATH: &'static str = "path_planner.toml";
}
