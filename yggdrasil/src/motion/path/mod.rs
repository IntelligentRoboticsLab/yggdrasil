//! This module implements pathfinding.

pub mod finding;
pub mod geometry;
pub mod obstacles;
pub mod planning;
pub mod visualization;

pub use planning::PathPlanner;

use std::f32::consts::PI;

use bevy::prelude::*;

use obstacles::{add_static_obstacles, obstacles_changed, update_colliders};
use visualization::{init_visualization, visualize_obstacles, visualize_path};

/// Plugin providing pathfinding capabilities.
pub struct PathPlugin;

impl Plugin for PathPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PathPlanner>()
            .add_systems(Startup, add_static_obstacles)
            .add_systems(PostStartup, init_visualization)
            .add_systems(Update, (update_colliders, visualize_obstacles).chain().run_if(obstacles_changed))
            .add_systems(Update, visualize_path.run_if(resource_changed::<PathPlanner>));
    }
}

/// Struct containing the configuration for the pathfinding.
#[derive(Copy, Clone, Resource)]
pub struct PathSettings {
    pub robot_radius: f32,
    pub ease_in: f32,
    pub ease_out: f32,
    pub start_tolerance: f32,
    pub target_tolerance: f32,
    pub perpendicular_tolerance: f32,
    pub angular_tolerance: f32,
    pub walk_speed: f32,
    pub turn_speed: f32,
    pub perpendicular_speed: f32,
    pub angular_speed: f32,
}

impl Default for PathSettings {
    fn default() -> Self {
        Self {
            robot_radius: 0.25,
            start_tolerance: 0.2,
            target_tolerance: 0.2,
            perpendicular_tolerance: 0.05,
            angular_tolerance: 0.1 * PI,
            ease_in: 0.25,
            ease_out: 0.25,
            walk_speed: 0.05,
            turn_speed: 0.2 * PI,
            perpendicular_speed: 0.02,
            angular_speed: 0.1 * PI,
        }
    }
}
