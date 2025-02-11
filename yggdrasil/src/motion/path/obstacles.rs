//! Obstacles and colliders.

use bevy::prelude::*;
use nalgebra as na;

use super::{
    finding::Colliders,
    geometry::{Circle, CircularArc, Point},
    planning::PathPlanner,
};

/// Adds initial obstacles to the scene.
pub fn add_static_obstacles(mut commands: Commands) {
    // Goalposts
    commands.spawn(Obstacle::from(Circle::new(na::point![-4.5, 0.8], 0.05)));
    commands.spawn(Obstacle::from(Circle::new(na::point![-4.5, -0.8], 0.05)));
    commands.spawn(Obstacle::from(Circle::new(na::point![4.5, 0.8], 0.05)));
    commands.spawn(Obstacle::from(Circle::new(na::point![4.5, -0.8], 0.05)));
}

/// Checks if any obstacles have been changed.
#[must_use]
pub fn obstacles_changed(obstacles: Query<&Obstacle, Changed<Obstacle>>) -> bool {
    !obstacles.is_empty()
}

/// Updates the [`Colliders`] based on the obstacles in the ECS (and reset [`Path`]).
pub fn update_colliders(mut planner: ResMut<PathPlanner>, obstacles: Query<&Obstacle>) {
    let radius = planner.config().robot_radius;

    let mut colliders = Colliders::new();

    for obstacle in &obstacles {
        obstacle.add_to_colliders(radius, &mut colliders);
    }

    planner.set_colliders(colliders);
}

/// Obstacle that the pathfinding navigates around.
#[derive(Clone, Component)]
pub enum Obstacle {
    Circle(Circle),
}

impl Obstacle {
    pub fn add_to_colliders(&self, radius: f32, colliders: &mut Colliders) {
        match self {
            &Obstacle::Circle(circle) => colliders.arcs.push(circle.dilate(radius).into()),
        }
    }

    pub fn vertices(&self, resolution: f32) -> impl Iterator<Item = Point> {
        match self {
            &Obstacle::Circle(c) => CircularArc::from(c).vertices(resolution),
        }
    }
}

impl From<Circle> for Obstacle {
    fn from(circle: Circle) -> Self {
        Self::Circle(circle)
    }
}
