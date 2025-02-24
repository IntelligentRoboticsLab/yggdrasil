//! Visualization of the pathfinding.

use bevy::prelude::*;

use crate::core::debug::DebugContext;

use super::{obstacles::Obstacle, planner::PathPlanner};

/// Number of vertices per circle.
const RESOLUTION: f32 = 64.0;
/// The height at which the obstacles are rendered.
const OBSTACLE_HEIGHT: f32 = 0.0001;
/// The height at which the path is rendered.
const PATH_HEIGHT: f32 = 0.0002;

pub fn init_visualization(dbg: DebugContext) {
    // TODO: i should fix this
    let _ = dbg;
    /*
    dbg.log_static(
        "pathfinding/obstacles",
        &rerun::Color::from_rgb(0, 0, 255),
    );

    dbg.log_static(
        "pathfinding/path",
        &rerun::Color::from_rgb(255, 0, 0),
    );
    */
}

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
pub fn visualize_path(dbg: DebugContext, planner: Res<PathPlanner>) {
    dbg.log(
        "pathfinding/path",
        &rerun::LineStrips3D::new([planner
            .path
            .as_ref()
            .into_iter()
            .flatten()
            .flat_map(|s| s.vertices(RESOLUTION))
            .map(|p| [p.x, p.y, PATH_HEIGHT])]),
    );
}
