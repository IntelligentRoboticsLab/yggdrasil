//! Visualization of the pathfinding.

use bevy::prelude::*;

use crate::core::debug::DebugContext;

use super::{obstacles::Obstacle, planning::Path};

/// Number of vertices per circle.
const RESOLUTION: f32 = 64.0;
/// The height at which the obstacles are rendered.
const OBSTACLE_HEIGHT: f32 = 0.0001;
/// The height at which the path is rendered.
const PATH_HEIGHT: f32 = 0.0002;

/// Visualizes the obstacles.
pub fn visualize_obstacles(dbg: DebugContext, obstacles: Query<&Obstacle>) {
    dbg.log(
        "pathfinding/obstacles",
        &rerun::LineStrips3D::new(obstacles.iter().map(|obstacle| {
            obstacle
                .vertices(RESOLUTION)
                .map(|p| [p.x, p.y, OBSTACLE_HEIGHT])
        })),
    );
}

/// Visualizes the path.
pub fn visualize_path(dbg: DebugContext, path: Res<Path>) {
    dbg.log(
        "pathfinding/path",
        &rerun::LineStrips3D::new([path
            .0
            .iter()
            .flat_map(|s| s.vertices(RESOLUTION))
            .map(|p| [p.x, p.y, PATH_HEIGHT])]),
    );
}
