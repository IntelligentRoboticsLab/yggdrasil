//! This module implements pathfinding.

pub mod finding;
pub mod geometry;
pub mod obstacles;
pub mod planner;

pub use finding::Target;
pub use obstacles::Obstacle;
pub use planner::PathPlanner;

use bevy::prelude::*;
use nalgebra as na;
use serde::{Deserialize, Serialize};

use crate::core::config::ConfigExt;
use crate::core::debug::DebugContext;
use crate::nao::Cycle;

use finding::Colliders;
use geometry::{Circle, LineSegment};

use odal::Config;

/// Number of vertices per circle.
const RESOLUTION: f32 = 64.0;
/// The height at which the obstacles are rendered.
const OBSTACLE_HEIGHT: f32 = 0.0001;
/// The height at which the path is rendered.
const PATH_HEIGHT: f32 = 0.0002;

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
    ///
    /// This controls the maximum distance the robot is allowed to be from the path before it
    /// starts side-stepping to correct the error.
    pub perpendicular_deadband: f32,
    /// The speed at which the perpendicular error gets corrected.
    pub perpendicular_speed: f32,
    /// The deadband of the angular error.
    ///
    /// This controls the maximum angle the robot is allowed to differ from the path before it
    /// starts adding a turn to correct the error.
    pub angular_deadband: f32,
    /// The speed at which the angular error gets corrected while walking.
    pub angular_speed: f32,
    /// The angular threshold at which the robot stops to turn.
    ///
    /// This is the angle at which the robot stops correcting its angle while walking and stands
    /// still to turn.
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

/// Adds initial obstacles to the scene.
pub fn add_static_obstacles(mut commands: Commands) {
    // Add goalposts
    commands.spawn(Obstacle::from(Circle::new(na::point![-4.5, 0.8], 0.05)));
    commands.spawn(Obstacle::from(Circle::new(na::point![-4.5, -0.8], 0.05)));
    commands.spawn(Obstacle::from(LineSegment::new(
        na::point![-4.5, 0.8],
        na::point![-5., 0.8],
    )));
    commands.spawn(Obstacle::from(LineSegment::new(
        na::point![-4.5, -0.8],
        na::point![-5., -0.8],
    )));
    commands.spawn(Obstacle::from(Circle::new(na::point![4.5, 0.8], 0.05)));
    commands.spawn(Obstacle::from(Circle::new(na::point![4.5, -0.8], 0.05)));
    commands.spawn(Obstacle::from(LineSegment::new(
        na::point![4.5, 0.8],
        na::point![5., 0.8],
    )));
    commands.spawn(Obstacle::from(LineSegment::new(
        na::point![4.5, -0.8],
        na::point![5., -0.8],
    )));
}

/// Checks if any obstacles have been changed.
#[must_use]
pub fn obstacles_changed(obstacles: Query<&Obstacle, Changed<Obstacle>>) -> bool {
    !obstacles.is_empty()
}

/// Updates the [`Colliders`] based on the obstacles in the ECS.
pub fn update_colliders(mut planner: ResMut<PathPlanner>, obstacles: Query<&Obstacle>) {
    let radius = planner.config().robot_radius;

    let mut colliders = Colliders::new();

    for obstacle in &obstacles {
        obstacle.add_to_colliders(radius, &mut colliders);
    }

    planner.set_colliders(colliders);
}

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
                .into_iter()
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
