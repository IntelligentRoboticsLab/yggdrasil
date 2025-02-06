//! Higher-level pathfinding capabilities.

use bevy::prelude::*;
use nalgebra as na;

use crate::{localization::RobotPose, motion::walk::engine::Step};

use super::{
    finding::{Pathfinding, Position},
    geometry::{Isometry, Segment},
    obstacles::Colliders,
    PathSettings,
};

/// Struct containing segments that make up a path.
#[derive(Default, Resource)]
pub struct Path {
    /// The segments that this path contains.
    pub segments: Vec<Segment>,
    /// Whether the path is considered suboptimal and should be recalculated.
    pub suboptimal: bool,
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
        let new = Path::new(pose.inner, target.0, &colliders, &settings, false);

        if !new.is_empty() {
            *path = new;
        } else {
            *path = Path::new(pose.inner, target.0, &Colliders::new(), &settings, true);
        }
    } else if path.suboptimal || !path.sync(pose.inner, &settings) {
        let new = Path::new(pose.inner, target.0, &colliders, &settings, false);

        if !new.is_empty() {
            *path = new;
        }
    }
}

impl Path {
    /// Create a new path based on the given pose, target, colliders, and settings.
    #[must_use]
    pub fn new(
        pose: Isometry,
        target: Option<Position>,
        colliders: &Colliders,
        settings: &PathSettings,
        suboptimal: bool,
    ) -> Self {
        if let Some(target) = target {
            let pathfinding = Pathfinding {
                start: pose.into(),
                goal: target,
                colliders,
                settings,
            };

            if let Some((path, _)) = pathfinding.path() {
                return Self {
                    segments: path,
                    suboptimal,
                };
            }
        }

        Self::default()
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
