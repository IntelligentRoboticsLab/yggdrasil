//! Higher-level pathfinding capabilities.

use bevy::prelude::*;
use nalgebra as na;

use crate::{localization::RobotPose, motion::walking_engine::step::Step};

use super::{
    finding::{Pathfinding, Position},
    geometry::{Isometry, Segment},
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
    /// Whether the pathfinding was able to ease in.
    pub ease_in: bool,
    /// Whether the pathfinding was able to ease out.
    pub ease_out: bool,
    /// Whether collisions were calculated.
    pub collisions: bool,
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
    // reset the target if we reached it
    if let Target(Some(position)) = *target {
        if na::distance(&position.to_point(), &pose.world_position()) <= settings.target_tolerance {
            *target = Target(None);
        }
    }

    if !path.ends_at(target.0, &settings) {
        // if the path doesn't go towards the target, find a new one.
        *path = Path::new(pose.inner, target.0, &colliders, &settings);
    } else if path.suboptimal || !path.sync(pose.inner, &settings) {
        // if the path is suboptimal or desynchronized, find a new one.
        let new = Path::new(pose.inner, target.0, &colliders, &settings);

        // make sure we don't overwrite the old path with a worse one.
        if !new.suboptimal && !new.is_empty() {
            *path = new;
        }
    }
}

impl Path {
    /// Finds a path, potentially falling back on suboptimal paths.
    #[must_use]
    pub fn new(
        pose: Isometry,
        target: Option<Position>,
        colliders: &Colliders,
        settings: &PathSettings
    ) -> Self {
        let Some(target) = target else {
            return Self::default();
        };

        let pathfinding = Pathfinding {
            start: pose.into(),
            goal: target,
            colliders,
            settings,
        };

        Self::find(pathfinding, true, true, true)
            .or_else(|| Self::find(pathfinding, false, true, true))
            .or_else(|| Self::find(pathfinding, true, false, true))
            .or_else(|| Self::find(pathfinding, false, false, true))
            .or_else(|| Self::find(pathfinding, true, true, false))
            .or_else(|| Self::find(pathfinding, false, true, false))
            .or_else(|| Self::find(pathfinding, true, false, false))
            .or_else(|| Self::find(pathfinding, false, false, false))
            .unwrap()
    }

    /// Finds a path with the given settings.
    #[must_use]
    fn find(
        mut pathfinding: Pathfinding,
        ease_in: bool,
        ease_out: bool,
        collisions: bool,
    ) -> Option<Self> {
        const EMPTY: &'static Colliders = &Colliders::new();

        if !ease_in {
            pathfinding.start.isometry()?;
            pathfinding.start = pathfinding.start.to_point().into();
        }

        if !ease_out {
            pathfinding.goal.isometry()?;
            pathfinding.goal = pathfinding.goal.to_point().into();
        }

        if !collisions {
            pathfinding.colliders = EMPTY;
        }

        let (segments, _) = pathfinding.path()?;

        Some(Self {
            segments,
            ease_in,
            ease_out,
            collisions,
            suboptimal: !(ease_in && ease_out && collisions),
        })
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
