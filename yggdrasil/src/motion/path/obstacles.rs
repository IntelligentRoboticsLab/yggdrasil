//! Obstacles and colliders.

use bevy::prelude::*;
use nalgebra as na;

use super::geometry::{Circle, CircularArc, Intersects, LineSegment, Point, Segment};

/// Adds initial obstacles to the scene.
pub fn add_static_obstacles(mut commands: Commands) {
    commands.spawn(Obstacle::from(Circle::origin(1.)));
    commands.spawn(Obstacle::from(Circle::new(na::point![2., -2.], 0.25)));
    commands.spawn(Obstacle::from(Circle::new(na::point![-1., -2.], 0.75)));
}

/// Checks if any obstacles have been changed.
#[must_use]
pub fn obstacles_changed(obstacles: Query<&Obstacle, Changed<Obstacle>>) -> bool {
    !obstacles.is_empty()
}

/// Updates the [`Colliders`] based on the obstacles in the ECS.
pub fn update_colliders(mut colliders: ResMut<Colliders>, obstacles: Query<&Obstacle>) {
    *colliders = Colliders::from_iter(&obstacles);
}

/// Obstacle that the pathfinding navigates around.
#[derive(Clone, Component)]
pub enum Obstacle {
    Circle(Circle),
}

impl Obstacle {
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

/// The colliders that the pathfinding navigates around, consists of circular arcs and line
/// segments.
#[derive(Clone, Default, Resource)]
pub struct Colliders {
    pub arcs: Vec<CircularArc>,
    pub lines: Vec<LineSegment>,
}

impl Colliders {
    /// Iterates over the segments that this struct contains.
    pub fn segments(&self) -> impl Iterator<Item = Segment> + '_ {
        let arcs = self.arcs.iter().copied().map(Into::into);
        let lines = self.lines.iter().copied().map(Into::into);

        arcs.chain(lines)
    }
}

impl<'a> FromIterator<&'a Obstacle> for Colliders {
    fn from_iter<T: IntoIterator<Item = &'a Obstacle>>(iter: T) -> Self {
        Self {
            arcs: iter
                .into_iter()
                .map(|obstacle| match obstacle {
                    &Obstacle::Circle(c) => c.dilate(0.5).into(),
                })
                .collect(),
            lines: Vec::new(),
        }
    }
}

impl Intersects<Segment> for &Colliders {
    type Intersection = bool;

    fn intersects(self, segment: Segment) -> bool {
        for other in self.segments() {
            if segment.intersects(other) {
                return true;
            }
        }

        false
    }
}
