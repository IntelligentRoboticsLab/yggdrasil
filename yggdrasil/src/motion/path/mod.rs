//! This module implements pathfinding.

pub mod finding;
pub mod geometry;
pub mod planning;
pub mod obstacles;

use bevy::prelude::*;

use finding::Path;
use obstacles::{add_static_obstacles, obstacles_changed, update_colliders, visualize_obstacles, Colliders};
use planning::{update_path, visualize_path};

/// Plugin providing pathfinding capabilities.
pub struct PathPlugin;

impl Plugin for PathPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<Colliders>()
            .init_resource::<Path>()
            .add_systems(Startup, add_static_obstacles)
            .add_systems(Update, update_colliders.run_if(obstacles_changed))
            .add_systems(Update, visualize_obstacles.run_if(resource_changed::<Colliders>))
            .add_systems(Update, update_path)
            .add_systems(Update, visualize_path.run_if(resource_changed::<Path>));
    }
}
