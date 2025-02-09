//! Higher-level pathfinding capabilities.

use bevy::prelude::*;

use crate::motion::walking_engine::step::Step;

use super::{
    finding::{Pathfinding, Position},
    geometry::{Isometry, LineSegment, Segment, Winding},
    obstacles::Colliders,
    PathSettings,
};

#[derive(Resource)]
pub struct PathPlanner {
    path: Option<Vec<Segment>>,
    target: Option<Position>,
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
    pub fn step(&mut self, start: Isometry) -> Option<Step> {
        let angular_tolerance = self.settings.angular_tolerance;
        let first = self.path(start.into()).first()?;
        let angle = Winding::shortest_distance(start.rotation.angle(), first.forward_at_start());

        if angle.abs() > angular_tolerance {
            Some(Step {
                forward: 0.,
                left: 0.,
                turn: 0.8 * angle.signum(),
            })
            } else {
            Some(Step {
                forward: 1.,
                left: 0.,
                turn: first.turn()
            })
        }
    }

    #[must_use]
    pub fn path(&mut self, start: Position) -> &mut Vec<Segment> {
        self.clear_target_if_reached(start);

        if self.ends_at_target() && self.begins_at_start(start) {
            self.trim_to_start(start);
            self.path.as_mut().unwrap()
        } else {
            self.path.insert(self.find_path(start))
        }
    }

    fn clear_target_if_reached(&mut self, start: Position) {
        let tolerance = self.settings.target_tolerance;
        self.target.take_if(|target| start.distance(*target) <= tolerance);
    }

    #[must_use]
    fn begins_at_start(&self, start: Position) -> bool {
        let Some(path) = self.path.as_ref() else {
            return false
        };

        let Some(first) = path.first() else {
            return self.target.is_none()
        };

        start.distance(first.start().into()) <= self.settings.tolerance
    }

    #[must_use]
    fn ends_at_target(&self) -> bool {
        let Some(path) = self.path.as_ref() else {
            return false
        };

        let Some(target) = self.target.as_ref() else {
            return path.is_empty()
        };

        let Some(last) = path.last() else {
            return false
        };

        target.distance(last.end().into()) <= self.settings.target_tolerance
    }

    fn trim_to_start(&mut self, start: Position) {
        let Some(path) = self.path.as_mut() else {
            return
        };

        let point = start.to_point();

        while let Some(first) = path.first_mut() {
            if start.distance(first.end().into()) > self.settings.tolerance {
                first.shorten(point);
                break
            }

            path.remove(0);
        }
    }

    #[must_use]
    pub fn find_path(&self, start: Position) -> Vec<Segment> {
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
        start: Position,
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
    pub fn fallback_path(&self, start: Position) -> Vec<Segment> {
        self
            .target
            .map(|target| LineSegment::new(start.to_point(), target.to_point()).into())
            .into_iter()
            .collect()
    }

    #[must_use]
    pub fn current_path(&self) -> Option<&Vec<Segment>> {
        self.path.as_ref()
    }

    pub fn target(&self) -> Option<Position> {
        self.target
    }

    pub fn set_target(&mut self, target: Option<Position>) {
        self.target = target;
        self.path = None;
    }

    #[must_use]
    pub fn colliders(&self) -> &Colliders {
        &self.colliders
    }

    pub fn set_colliders(&mut self, colliders: Colliders) {
        self.colliders = colliders;
        self.path = None;
    }

    #[must_use]
    pub fn settings(&self) -> &PathSettings {
        &self.settings
    }

    pub fn set_settings(&mut self, settings: PathSettings) {
        self.settings = settings;
        self.path = None;
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
    pub fn ease_in(self) -> bool {
        match self {
            Self::In | Self::InOut => true,
            _ => false,
        }
    }

    pub fn ease_out(self) -> bool {
        match self {
            Self::Out | Self::InOut => true,
            _ => false,
        }
    }
}
