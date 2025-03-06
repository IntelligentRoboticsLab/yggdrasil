//! Obstacles and colliders.

use bevy::prelude::*;

use super::{
    finding::Colliders,
    geometry::{Circle, CircularArc, Point},
};

/// Obstacle that the pathfinding navigates around.
#[derive(Clone, Component)]
pub enum Obstacle {
    Circle(Circle),
}

impl Obstacle {
    /// Adds this obstacle to the given colliders (based on the robot radius).
    pub fn add_to_colliders(&self, radius: f32, colliders: &mut Colliders) {
        match self {
            &Obstacle::Circle(circle) => colliders.arcs.push(circle.dilate(radius).into()),
        }
    }

    /// Gets the vertices that represent this obstacle.
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
