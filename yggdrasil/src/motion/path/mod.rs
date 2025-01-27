//! This module implements pathfinding.

pub mod finding;
pub mod geometry;
pub mod obstacles;
pub mod planning;
pub mod visualization;

use std::f32::consts::PI;

use bevy::prelude::*;

use obstacles::{add_static_obstacles, obstacles_changed, update_colliders, Colliders};
use planning::{update_target_path_and_step, Path, StepAlongPath, Target};
use visualization::{visualize_obstacles, visualize_path};

/// Plugin providing pathfinding capabilities.
pub struct PathPlugin;

impl Plugin for PathPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Colliders>()
            .init_resource::<Path>()
            .init_resource::<Target>()
            .init_resource::<StepAlongPath>()
            .init_resource::<PathSettings>()
            .add_systems(Startup, add_static_obstacles)
            .add_systems(Update, update_target_path_and_step)
            .add_systems(Update, update_colliders.run_if(obstacles_changed))
            .add_systems(Update, visualize_path.run_if(resource_changed::<Path>))
            .add_systems(
                Update,
                visualize_obstacles.run_if(resource_changed::<Colliders>),
            );
    }
}

/// Struct containing the configuration for the pathfinding.
#[derive(Copy, Clone, Resource)]
pub struct PathSettings {
    /// The radius of the robot (minimum distance it keeps from obstacles).
    pub robot_radius: f32,
    /// The maximum distance the robot is allowed to be from the path before it is considered
    /// desynchronized.
    pub tolerance: f32,
    /// The maximum angular distance the robot is allowed to be from the path before it is
    /// considered desynchronized.
    pub angular_tolerance: f32,
    /// The radius ou the arc to ease into the path from the start counterclockwise.
    pub ccw_ease_in: f32,
    /// The radius ou the arc to ease into the path from the start clockwise.
    pub cw_ease_in: f32,
    /// The radius of the arc to ease out of the path into the goal counterclockwise.
    pub ccw_ease_out: f32,
    /// The radius of the arc to ease out of the path into the goal clockwise.
    pub cw_ease_out: f32,
}

impl Default for PathSettings {
    fn default() -> Self {
        Self {
            robot_radius: 0.25,
            tolerance: 0.1,
            angular_tolerance: 0.1 * PI,
            ccw_ease_in: 1.0,
            cw_ease_in: 1.0,
            ccw_ease_out: 1.0,
            cw_ease_out: 1.0,
        }
    }
}
