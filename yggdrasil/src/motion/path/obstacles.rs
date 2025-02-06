//! Obstacles and colliders.

use bevy::prelude::*;
use nalgebra as na;

use super::{
    geometry::{Circle, CircularArc, Intersects, LineSegment, Point, Segment},
    planning::Path,
    PathSettings,
};

/// Adds initial obstacles to the scene.
pub fn add_static_obstacles(mut commands: Commands) {
    //commands.spawn(Obstacle::from(Circle::origin(1.)));
    commands.spawn(Obstacle::from(Circle::new(na::point![-2.5, 2.], 0.25)));
    //commands.spawn(Obstacle::from(Circle::new(na::point![-1., -2.], 0.75)));
}

/// Checks if any obstacles have been changed.
#[must_use]
pub fn obstacles_changed(obstacles: Query<&Obstacle, Changed<Obstacle>>) -> bool {
    !obstacles.is_empty()
}

/// Updates the [`Colliders`] based on the obstacles in the ECS (and reset [`Path`]).
pub fn update_colliders(mut colliders: ResMut<Colliders>, mut path: ResMut<Path>, settings: Res<PathSettings>, obstacles: Query<&Obstacle>) {
    *colliders = Colliders::from_obstacles(&obstacles, &settings);
    *path = Path::default();
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
    pub fn new() -> Self {
        Self {
            arcs: Vec::new(),
            lines: Vec::new(),
        }
    }

    /// Creates a new [`Colliders`] from the given obstacles and settings.
    pub fn from_obstacles<'a, T: IntoIterator<Item = &'a Obstacle>>(iter: T, settings: &PathSettings) -> Self {
        Self {
            arcs: iter
                .into_iter()
                .map(|obstacle| match obstacle {
                    &Obstacle::Circle(c) => c.dilate(settings.robot_radius).into(),
                })
                .collect(),
            lines: Vec::new(),
        }
    }

    /// Iterates over the segments that this struct contains.
    pub fn segments(&self) -> impl Iterator<Item = Segment> + '_ {
        let arcs = self.arcs.iter().copied().map(Into::into);
        let lines = self.lines.iter().copied().map(Into::into);

        arcs.chain(lines)
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
