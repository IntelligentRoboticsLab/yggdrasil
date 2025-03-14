//! Higher-level pathfinding capabilities.

use bevy::prelude::*;

use crate::motion::walking_engine::step::Step;

use super::{
    finding::{Colliders, Pathfinding, Target},
    geometry::{Isometry, LineSegment, Point, Segment, Winding},
    PathConfig,
};

/// Struct that maintains the current path being followed.
#[derive(Resource)]
pub struct PathPlanner {
    pub path: Option<Vec<Segment>>,
    pub target: Option<Target>,
    colliders: Colliders,
    config: PathConfig,
}

impl PathPlanner {
    /// Creates a new [`PathPlanner`].
    #[must_use]
    pub fn new(config: PathConfig) -> Self {
        Self {
            path: Some(Vec::new()),
            target: None,
            colliders: Colliders::new(),
            config,
        }
    }

    /// Returns whether a path has been found and is empty (i.e., because we reached the target).
    #[must_use]
    pub fn reached_target(&self) -> bool {
        self.path.as_ref().is_some_and(Vec::is_empty)
    }

    /// Returns the next step that would move the robot along the current path.
    #[must_use]
    pub fn step(&mut self, start: Isometry) -> Option<Step> {
        let PathConfig {
            perpendicular_deadband,
            perpendicular_speed,
            angular_deadband,
            angular_speed,
            stop_and_turn_threshold,
            walk_speed,
            turn_speed,
            walking_turn_speed,
            ..
        } = self.config;

        // Recalculate the path and get the first segment.
        let first = self.path(start.into()).first()?;
        let point = start.translation.vector.into();

        // Calculate the error between the segment being followed and the current position.
        let angular_error =
            Winding::shortest_distance(start.rotation.angle(), first.forward_at_start());

        // Clamp and deadband the angular correction to apply.
        let angular_correction = if angular_error.abs() <= angular_deadband {
            0.
        } else {
            angular_error.clamp(-angular_speed, angular_speed)
        };

        if angular_error.abs() <= stop_and_turn_threshold {
            // Calculate the perpendicular error (after projecting forward).
            let perpendicular_error =
                first.signed_distance(point) + walk_speed * angular_error.sin();

            // Clamp and deadband the perpendicular correction to apply.
            let left = if perpendicular_error.abs() <= perpendicular_deadband {
                0.
            } else {
                (perpendicular_error / angular_error.cos())
                    .clamp(-perpendicular_speed, perpendicular_speed)
            };

            // Keep walking forward if we're walking somewhat in the right direction.
            Some(Step {
                forward: walk_speed,
                left,
                turn: walking_turn_speed * first.turn() + angular_correction,
            })
        } else {
            // Otherwise, stop and turn.
            Some(Step {
                forward: 0.,
                left: 0.,
                turn: angular_error.clamp(-turn_speed, turn_speed),
            })
        }
    }

    /// Calculates a new path if we or the target aren't on it, and trims it to the start.
    #[must_use]
    pub fn path(&mut self, start: Target) -> &mut Vec<Segment> {
        let point = start.to_point();

        if self.ends_at_target(start) {
            self.trim_to_start(point);

            if self.begins_at_start(start) {
                return self.path.as_mut().unwrap();
            }
        }

        self.path.insert(self.find_path(start))
    }

    /// Checks whether the path starts close enough to the start.
    #[must_use]
    fn begins_at_start(&self, start: Target) -> bool {
        let Some(path) = &self.path else { return false };

        let Some(first) = path.first() else {
            return match self.target {
                Some(target) => start.distance(target) <= self.config.target_tolerance,
                None => true,
            };
        };

        start.distance(first.start().into()) <= self.config.start_tolerance
    }

    /// Checks whether the path ends close enough to the target.
    #[must_use]
    fn ends_at_target(&self, start: Target) -> bool {
        let Some(path) = &self.path else { return false };

        let Some(target) = &self.target else {
            return path.is_empty();
        };

        let end = match path.last() {
            Some(last) => last.end().into(),
            None => start,
        };

        target.distance(end) <= self.config.target_tolerance
    }

    /// Trims the path to the current start (i.e., only keeps the remaining path).
    fn trim_to_start(&mut self, start: Point) {
        let Some(path) = self.path.as_mut() else {
            return;
        };

        while let Some(first) = path.first_mut() {
            first.trim(start);

            if first.beyond(start) {
                path.remove(0);
            } else {
                break;
            }
        }
    }

    /// Finds a new path, disabling easing in/out and eventually collisions if no path can otherwise
    /// be found.
    #[must_use]
    pub fn find_path(&self, start: Target) -> Vec<Segment> {
        self.find_path_with(start, Ease::InOut, true)
            .or_else(|| self.find_path_with(start, Ease::Out, true))
            .or_else(|| self.find_path_with(start, Ease::In, true))
            .or_else(|| self.find_path_with(start, Ease::None, true))
            .or_else(|| self.find_path_with(start, Ease::InOut, false))
            .or_else(|| self.find_path_with(start, Ease::Out, false))
            .or_else(|| self.find_path_with(start, Ease::In, false))
            .unwrap_or_else(|| self.fallback_path(start))
    }

    /// Finds a path to the target with the given settings.
    #[must_use]
    pub fn find_path_with(
        &self,
        start: Target,
        ease: Ease,
        collisions: bool,
    ) -> Option<Vec<Segment>> {
        let Some(target) = self.target else {
            return Some(Vec::new());
        };

        let config = self.config();

        let half_distance = 0.5 * start.distance(target);

        // If we're far away enough to ease in, try to do so (and fail if the start is not an
        // isometry).
        let start = if ease.ease_in()
            && (half_distance >= config.ease_in_radius + config.ease_out_radius)
        {
            start
        } else {
            start.isometry_to_point()?
        };

        // If we're far away enough to ease out, try to do so (and fail if the target is not an
        // isometry).
        let target = if ease.ease_out() && (half_distance >= config.ease_out_radius) {
            target
        } else {
            target.isometry_to_point()?
        };

        // If collisions are disabled, use a dummy.
        let colliders = if collisions {
            self.colliders()
        } else {
            &Colliders::new()
        };

        let pathfinding = Pathfinding {
            start,
            target,
            colliders,
            config,
        };

        // Find the path (and discard the cost since we don't use it).
        Some(pathfinding.path()?.0)
    }

    /// The shortest path between two points is a straight line, returns that one.
    #[must_use]
    pub fn fallback_path(&self, start: Target) -> Vec<Segment> {
        self.target
            .map(|target| LineSegment::new(start.to_point(), target.to_point()).into())
            .into_iter()
            .collect()
    }

    /// Returns a reference to the colliders.
    #[must_use]
    pub fn colliders(&self) -> &Colliders {
        &self.colliders
    }

    /// Sets the colliders and invalidates the path if the colliders changed.
    pub fn set_colliders(&mut self, colliders: Colliders) {
        if self.colliders != colliders {
            self.colliders = colliders;
            self.path = None;
        }
    }

    /// Returns a reference to the config.
    #[must_use]
    pub fn config(&self) -> &PathConfig {
        &self.config
    }

    /// Sets the config and invalidates the path if the config changed.
    pub fn set_config(&mut self, config: PathConfig) {
        if self.config != config {
            self.config = config;
            self.path = None;
        }
    }
}

/// Whether we should ease in, ease out, both, or neither.
#[derive(Copy, Clone)]
pub enum Ease {
    In,
    Out,
    InOut,
    None,
}

impl Ease {
    /// Returns whether we should ease in.
    #[must_use]
    pub fn ease_in(self) -> bool {
        matches!(self, Self::In | Self::InOut)
    }

    /// Returns whether we should ease out.
    #[must_use]
    pub fn ease_out(self) -> bool {
        matches!(self, Self::Out | Self::InOut)
    }
}
