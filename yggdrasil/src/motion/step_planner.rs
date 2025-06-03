use super::{
    path_finding::{self, Obstacle},
    walking_engine::step::Step,
};
use crate::{core::debug::DebugContext, localization::RobotPose, nao::Cycle};
use bevy::prelude::*;
use nalgebra::{Isometry2, Point2, UnitComplex, Vector2};
use rerun::{FillMode, LineStrip3D};

const WALK_SPEED: f32 = 0.045;
const TURN_SPEED: f32 = 0.3;

const DISTANCE: f32 = 0.05;
const TOLERANCE: f32 = 0.1;
const MOMENTUM: f32 = 0.9;
const COMPENSATION: f32 = 0.1;

/// Plugin that adds systems and resources for planning robot steps.
pub(super) struct StepPlannerPlugin;

impl Plugin for StepPlannerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StepPlanner>()
            .add_systems(PostStartup, setup_path_visualizer)
            .add_systems(PostUpdate, log_planned_path);
    }
}

#[derive(Debug, Clone, Resource)]
pub struct StepPlanner {
    waypoints: Vec<Point2<f32>>,
    rotation: Option<f32>,
    momentum: Vector2<f32>,
}

impl Default for StepPlanner {
    fn default() -> Self {
        Self {
            waypoints: vec![
                Point2::new(-1., -1.),
                Point2::new( 1., -1.),
                Point2::new( 1.,  1.),
                Point2::new(-1.,  1.),
            ],
            rotation: Some(0.),
            momentum: Vector2::new(0., 0.),
        }
    }
}

impl StepPlanner {
    pub fn plan(&mut self, pose: &RobotPose) -> Option<Step> {
        let position = pose.world_position();

        self.pop_waypoint_if_reached(&position);

        if let Some(direction) = self.walking_direction(&position) {
            let direction = pose.inner.inverse_transform_vector(&direction);

            Some(Step {
                forward: WALK_SPEED * direction.x,
                left: WALK_SPEED * direction.y,
                turn: direction.y.atan2(direction.x).min(TURN_SPEED).max(-TURN_SPEED),
            })
        } else {
            let error = self.rotation? - pose.world_rotation();

            (error > TOLERANCE).then_some(Step {
                forward: 0.,
                left: 0.,
                turn: error.min(TURN_SPEED).max(-TURN_SPEED),
            })
        }
    }

    fn pop_waypoint_if_reached(&mut self, position: &Point2<f32>) {
        if let Some(first) = self.waypoints.first() {
            if (first - position).norm() <= DISTANCE {
                self.waypoints.remove(0);
            }
        }
    }

    fn walking_direction(&mut self, position: &Point2<f32>) -> Option<Vector2<f32>> {
        let first = self.waypoints.first()?;

        let mut direction = first - position;

        if let Some(second) = self.waypoints.get(1) {
            direction = direction.normalize() - COMPENSATION * (second - first).normalize();
        }

        self.momentum *= MOMENTUM;
        self.momentum += (1. - MOMENTUM) * direction.normalize();

        Some(self.momentum)
    }

    #[deprecated]
    pub fn set_absolute_target(&mut self, target: Target) {
        let _ = target;
    }

    #[deprecated]
    pub fn set_absolute_target_if_unset(&mut self, target: Target) {
        let _ = target;
    }

    #[must_use]
    #[deprecated]
    pub fn current_absolute_target(&self) -> Option<&Target> {
        None
    }

    #[deprecated]
    pub fn clear_target(&mut self) {
    }

    #[deprecated]
    pub fn add_dynamic_obstacle(&mut self, obstacle: DynamicObstacle, merge_distance: f32) {
        let _ = (obstacle, merge_distance);
    }

    #[must_use]
    #[deprecated]
    pub fn reached_target(&self) -> bool {
        false
    }

    #[must_use]
    #[deprecated]
    pub fn has_target(&self) -> bool {
        true
    }

}

#[derive(Clone, Copy, PartialEq, Debug)]
#[deprecated]
pub struct Target {
    pub position: Point2<f32>,
    pub rotation: Option<UnitComplex<f32>>,
}

impl From<Point2<f32>> for Target {
    fn from(position: Point2<f32>) -> Self {
        Target {
            position,
            rotation: None,
        }
    }
}

#[derive(Debug, Clone)]
#[deprecated]
pub struct DynamicObstacle {
    pub obs: Obstacle,
    pub ttl: std::time::Instant,
}

fn setup_path_visualizer(dbg: DebugContext) {
    dbg.log_with_cycle(
        "field/path",
        Cycle::default(),
        &rerun::LineStrips3D::update_fields()
            .with_colors([(66, 135, 245)])
            .with_radii([0.01]),
    );
}

fn log_planned_path(
    dbg: DebugContext,
    cycle: Res<Cycle>,
    robot_pose: Res<RobotPose>,
    step_planner: ResMut<StepPlanner>,
) {
    dbg.log_with_cycle(
        "field/path",
        *cycle,
        &rerun::LineStrips3D::update_fields().with_strips([LineStrip3D::from_iter(
            std::iter::once(&robot_pose.world_position())
                .chain(&step_planner.waypoints)
                .map(|point| (point.x, point.y, 0.05)),
        )]),
    );
}
