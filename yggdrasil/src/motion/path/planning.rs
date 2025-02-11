//! Higher-level pathfinding capabilities.

use bevy::prelude::*;

use crate::motion::walking_engine::step::Step;

use super::{
    finding::{Colliders, Pathfinding, Target},
    geometry::{Isometry, LineSegment, Point, Segment, Winding},
    PathSettings,
};

#[derive(Resource)]
pub struct PathPlanner {
    pub path: Option<Vec<Segment>>,
    pub target: Option<Target>,
    colliders: Colliders,
    settings: PathSettings,
}

impl PathPlanner {
    #[must_use]
    pub fn new(settings: PathSettings) -> Self {
        Self {
            path: Some(Vec::new()),
            target: None,
            colliders: Colliders::new(),
            settings,
        }
    }

    #[must_use]
    pub fn reached_target(&self) -> bool {
        match &self.path {
            Some(path) => path.is_empty(),
            None => false,
        }
    }

    #[must_use]
    pub fn step(&mut self, start: Isometry) -> Option<Step> {
        let PathSettings {
            perpendicular_tolerance,
            angular_tolerance,
            walk_speed,
            turn_speed,
            walking_turn_speed,
            perpendicular_speed,
            angular_speed,
            ..
        } = self.settings;

        let point = start.translation.vector.into();
        let first = self.path(start.into()).first()?;

        let perpendicular_correction = {
            let error = first.signed_distance(point);

            if error.abs() > perpendicular_tolerance {
                error.clamp(-perpendicular_speed, perpendicular_speed)
            } else {
                0.
            }
        };

        let angular_error =
            Winding::shortest_distance(start.rotation.angle(), first.forward_at_start());

        let angular_correction = angular_error.clamp(-angular_speed, angular_speed);

        if angular_error.abs() <= angular_tolerance {
            Some(Step {
                forward: walk_speed,
                left: perpendicular_correction,
                turn: walking_turn_speed * first.turn() + angular_correction,
            })
        } else {
            Some(Step {
                forward: 0.,
                left: perpendicular_correction,
                turn: angular_error.clamp(-turn_speed, turn_speed),
            })
        }
    }

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

    #[must_use]
    fn begins_at_start(&self, start: Target) -> bool {
        let Some(path) = &self.path else { return false };

        let Some(first) = path.first() else {
            return match self.target {
                Some(target) => start.distance(target) <= self.settings.target_tolerance,
                None => true,
            };
        };

        start.distance(first.start().into()) <= self.settings.start_tolerance
    }

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

        target.distance(end) <= self.settings.target_tolerance
    }

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

    #[must_use]
    pub fn find_path(&self, start: Target) -> Vec<Segment> {
        self.find_path_with(start, Ease::InOut, true)
            .or_else(|| self.find_path_with(start, Ease::In, true))
            .or_else(|| self.find_path_with(start, Ease::Out, true))
            .or_else(|| self.find_path_with(start, Ease::None, true))
            .or_else(|| self.find_path_with(start, Ease::InOut, false))
            .or_else(|| self.find_path_with(start, Ease::In, false))
            .or_else(|| self.find_path_with(start, Ease::Out, false))
            .unwrap_or_else(|| self.fallback_path(start))
    }

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

        let settings = self.settings();

        let half_distance = 0.5 * start.distance(target);

        let start = if ease.ease_in() && (half_distance >= settings.ease_in + settings.ease_out) {
            start
        } else {
            start.isometry_to_point()?
        };

        let target = if ease.ease_out() && (half_distance >= settings.ease_out) {
            target
        } else {
            target.isometry_to_point()?
        };

        let colliders = if collisions {
            self.colliders()
        } else {
            &Colliders::new()
        };

        let pathfinding = Pathfinding {
            start,
            target,
            colliders,
            settings,
        };

        Some(pathfinding.path()?.0)
    }

    #[must_use]
    pub fn fallback_path(&self, start: Target) -> Vec<Segment> {
        self.target
            .map(|target| LineSegment::new(start.to_point(), target.to_point()).into())
            .into_iter()
            .collect()
    }

    #[must_use]
    pub fn colliders(&self) -> &Colliders {
        &self.colliders
    }

    pub fn set_colliders(&mut self, colliders: Colliders) {
        if self.colliders != colliders {
            self.colliders = colliders;
            self.path = None;
        }
    }

    #[must_use]
    pub fn settings(&self) -> &PathSettings {
        &self.settings
    }

    pub fn set_settings(&mut self, settings: PathSettings) {
        if self.settings != settings {
            self.settings = settings;
            self.path = None;
        }
    }
}

impl Default for PathPlanner {
    fn default() -> Self {
        Self::new(PathSettings::default())
    }
}

#[derive(Copy, Clone)]
pub enum Ease {
    In,
    Out,
    InOut,
    None,
}

impl Ease {
    #[must_use]
    pub fn ease_in(self) -> bool {
        matches!(self, Self::In | Self::InOut)
    }

    #[must_use]
    pub fn ease_out(self) -> bool {
        matches!(self, Self::Out | Self::InOut)
    }
}
