//! Visualization of the pathfinding.

use bevy::prelude::*;

use crate::core::debug::DebugContext;
use crate::nao::Cycle;

use super::{obstacles::Obstacle, planner::PathPlanner};

/// Number of vertices per circle.
const RESOLUTION: f32 = 64.0;
/// The height at which the obstacles are rendered.
const OBSTACLE_HEIGHT: f32 = 0.0001;
/// The height at which the path is rendered.
const PATH_HEIGHT: f32 = 0.0002;

/// Initializes the visualization.
pub fn init_visualization(dbg: DebugContext) {
    dbg.log_with_cycle(
        "pathfinding/obstacles",
        Cycle::default(),
        &rerun::LineStrips3D::update_fields().with_colors([(0, 0, 255)]),
    );
    dbg.log_with_cycle(
        "pathfinding/path",
        Cycle::default(),
        &rerun::LineStrips3D::update_fields().with_colors([(255, 0, 0)]),
    );
}

/// Visualizes the obstacles.
pub fn visualize_obstacles(dbg: DebugContext, obstacles: Query<&Obstacle>, cycle: Res<Cycle>) {
    dbg.log_with_cycle(
        "pathfinding/obstacles",
        *cycle,
        &rerun::LineStrips3D::update_fields().with_strips(obstacles.iter().map(|obstacle| {
            obstacle
                .vertices(RESOLUTION)
                .map(|p| [p.x, p.y, OBSTACLE_HEIGHT])
        })),
    );
}

/// Visualizes the path.
pub fn visualize_path(dbg: DebugContext, planner: Res<PathPlanner>, cycle: Res<Cycle>) {
    dbg.log_with_cycle(
        "pathfinding/path",
        *cycle,
        &rerun::LineStrips3D::update_fields().with_strips([planner
            .path
            .as_ref()
            .into_iter()
            .flatten()
            .flat_map(|s| s.vertices(RESOLUTION))
            .map(|p| [p.x, p.y, PATH_HEIGHT])]),
    );
}
