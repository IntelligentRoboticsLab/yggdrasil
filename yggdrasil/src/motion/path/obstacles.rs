//! Obstacles and colliders.

use bevy::prelude::*;

use super::{
    finding::Colliders,
    geometry::{Circle, CircularArc, LineSegment, Point},
};

/// Obstacle that the pathfinding navigates around.
#[derive(Clone, Component)]
pub enum Obstacle {
    Circle(Circle),
    LineSegment(LineSegment),
}

impl Obstacle {
    /// Adds this obstacle to the given colliders (based on the robot radius).
    pub fn add_to_colliders(&self, radius: f32, colliders: &mut Colliders) {
        match *self {
            Obstacle::Circle(circle) => colliders.arcs.push(circle.dilate(radius).into()),
            Obstacle::LineSegment(line) => colliders.lines.push(line),
        }
    }

    /// Gets the vertices that represent this obstacle.
    #[must_use]
    pub fn vertices(&self, resolution: f32) -> impl IntoIterator<Item = Point> {
        match *self {
            Obstacle::Circle(c) => CircularArc::from(c).vertices(resolution).collect(),
            Obstacle::LineSegment(l) => vec![l.start, l.end],
        }
    }
}

impl From<Circle> for Obstacle {
    fn from(circle: Circle) -> Self {
        Self::Circle(circle)
    }
}

impl From<LineSegment> for Obstacle {
    fn from(line: LineSegment) -> Self {
        Self::LineSegment(line)
    }
}
