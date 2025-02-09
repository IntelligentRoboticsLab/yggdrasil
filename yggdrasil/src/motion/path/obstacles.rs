//! Obstacles and colliders.

use bevy::prelude::*;
use nalgebra as na;

use super::{
    finding::Colliders, geometry::{Circle, CircularArc, Point}, planning::PathPlanner, PathSettings
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
pub fn update_colliders(
    mut planner: ResMut<PathPlanner>,
    obstacles: Query<&Obstacle>,
) {
    let colliders = Obstacle::into_colliders(&obstacles, planner.settings());
    planner.set_colliders(colliders);
}

/// Obstacle that the pathfinding navigates around.
#[derive(Clone, Component)]
pub enum Obstacle {
    Circle(Circle),
}

impl Obstacle {
    /// Creates a new [`Colliders`] from the given obstacles and settings.
    pub fn into_colliders<'a, T: IntoIterator<Item = &'a Obstacle>>(
        iter: T,
        settings: &PathSettings,
    ) -> Colliders {
        Colliders {
            arcs: iter
                .into_iter()
                .map(|obstacle| match obstacle {
                    &Obstacle::Circle(c) => c.dilate(settings.robot_radius).into(),
                })
                .collect(),
            lines: Vec::new(),
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
