//! Higher-level pathfinding capabilities.

use bevy::prelude::*;
use nalgebra as na;

use crate::{localization::RobotPose, motion::walk::engine::Step};

use super::{
    finding::{Pathfinding, Position},
    geometry::{Isometry, LineSegment, Segment},
    obstacles::Colliders,
    PathSettings,
};

/// Struct containing segments that make up a path.
#[derive(Resource)]
pub struct Path {
    /// The segments that this path contains.
    pub segments: Vec<Segment>,
    /// Whether the path is considered suboptimal and should be recalculated.
    pub suboptimal: bool,
    /// Whether collisions were calculated.
    pub collisions: bool,
    /// Whether the pathfinding was able to ease in.
    pub ease_in: bool,
    /// Whether the pathfinding was able to ease out.
    pub ease_out: bool,
}

/// The target to walk to.
#[derive(Copy, Clone, Default, Resource)]
pub struct Target(pub Option<Position>);

/// Updates the [`Path`] and [`Target`] resources.
pub fn update_path(
    mut path: ResMut<Path>,
    mut target: ResMut<Target>,
    pose: Res<RobotPose>,
    colliders: Res<Colliders>,
    settings: Res<PathSettings>,
) {
    if let Target(Some(position)) = *target {
        if na::distance(&position.to_point(), &pose.world_position()) <= settings.target_tolerance {
            *target = Target(None);
        }
    }

    if !path.ends_at(target.0, &settings) {
        if let Target(Some(target)) = *target {
            let new = Path::new(pose.inner, target, &colliders, &settings);

            if !new.is_empty() && !new.suboptimal {
                *path = new;
            }
        }
    } else if path.suboptimal || !path.sync(pose.inner, &settings) {
        if let Target(Some(target)) = *target {
            let new = Path::new(pose.inner, target, &colliders, &settings);

            if !new.is_empty() {
                *path = new;
            }
        }
    }
}

impl Path {
    /// Create a new path based on the given pose, target, colliders, and settings.
    #[must_use]
    pub fn new(
        pose: Isometry,
        target: Position,
        colliders: &Colliders,
        settings: &PathSettings,
    ) -> Self {
        let mut pathfinding = Pathfinding {
            start: pose.into(),
            goal: target,
            colliders,
            settings,
        };

        let mut path = Self::default();
        let empty = Colliders::new();

        if let Some((segments, _)) = pathfinding.path() {
            path.segments = segments;
            return path;
        }

        pathfinding.start = pathfinding.start.to_point().into();
        path.suboptimal = true;
        path.ease_in = false;

        if let Some((segments, _)) = pathfinding.path() {
            path.segments = segments;
            return path;
        }

        if pathfinding.goal.isometry().is_some() {
            pathfinding.start = pose.into();
            pathfinding.goal = target.to_point().into();
            path.ease_in = true;
            path.ease_out = false;

            if let Some((segments, _)) = pathfinding.path() {
                path.segments = segments;
                return path;
            }

            pathfinding.start = pathfinding.start.to_point().into();
            path.ease_in = false;

            if let Some((segments, _)) = pathfinding.path() {
                path.segments = segments;
                return path;
            }
        }

        pathfinding.start = pose.into();
        pathfinding.goal = target;
        pathfinding.colliders = &empty;
        path.ease_in = true;
        path.ease_out = true;
        path.collisions = false;

        if let Some((segments, _)) = pathfinding.path() {
            path.segments = segments;
            return path;
        }

        path.ease_in = false;
        path.ease_out = false;
        path.segments = vec![
            LineSegment::new(pathfinding.start.to_point(), pathfinding.goal.to_point()).into(),
        ];

        return path;
    }

    /// Returns whether the path is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Returns the first segment in the path.
    #[must_use]
    pub fn first(&self) -> Option<Segment> {
        self.segments.first().copied()
    }

    /// Returns the last segment in the path.
    #[must_use]
    pub fn last(&self) -> Option<Segment> {
        self.segments.last().copied()
    }

    /// Returns the step required to follow the path.
    #[must_use]
    pub fn step(&self) -> Option<Step> {
        self.first().map(|first| Step {
            forward: 1.,
            left: 0.,
            turn: first.turn(),
        })
    }

    /// Returns whether the path ends at the given target.
    #[must_use]
    pub fn ends_at(&self, target: Option<Position>, settings: &PathSettings) -> bool {
        let Some(target) = target else {
            return self.is_empty();
        };

        let Some(last) = self.last() else {
            return false;
        };

        let target_point = target.to_point();
        let target_angle = target.isometry().map(|isometry| isometry.rotation.angle());

        let point = last.end();
        let angle = last.forward_at_end();

        let ok_distance = na::distance(&point, &target_point) <= settings.tolerance;
        let ok_angle = match target_angle {
            Some(target_angle) => (target_angle - angle).abs() <= settings.angular_tolerance,
            None => true,
        };

        ok_distance && ok_angle
    }

    /// Shortens the path to the point the robot is located, returning whether the robot is
    /// desynchronized (i.e., the robot is too far from the path).
    pub fn sync(&mut self, pose: Isometry, settings: &PathSettings) -> bool {
        let point = pose.translation.vector.into();
        let angle = pose.rotation.angle();

        loop {
            if self.is_empty() {
                return true;
            }

            let segment = &mut self.segments[0];

            if na::distance(&point, &segment.end()) <= settings.tolerance {
                self.segments.remove(0);
                continue;
            }

            segment.shorten(point);

            let ok_distance = na::distance(&point, &segment.start()) <= settings.tolerance;
            let ok_angle = (segment.forward_at_start() - angle).abs() <= settings.angular_tolerance;

            return ok_distance && ok_angle;
        }
    }
}

impl Default for Path {
    fn default() -> Self {
        Self {
            segments: Vec::new(),
            suboptimal: false,
            collisions: true,
            ease_in: true,
            ease_out: true,
        }
    }
}
