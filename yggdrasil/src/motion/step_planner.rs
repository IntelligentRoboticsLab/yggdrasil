use super::{
    path_finding::{self, Obstacle},
    walking_engine::step::Step,
};
use crate::{core::debug::DebugContext, localization::RobotPose, nao::Cycle};
use bevy::prelude::*;
use nalgebra::{Isometry, Point2, UnitComplex, Vector2};
use rerun::{FillMode, LineStrip3D};
use std::time::Instant;

const TURN_SPEED: f32 = 0.2;
const WALK_SPEED: f32 = 0.045;

/// Plugin that adds systems and resources for planning robot steps.
pub(super) struct StepPlannerPlugin;

impl Plugin for StepPlannerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StepPlanner>();
        app.add_systems(
            PostStartup,
            (setup_path_visualizer, setup_dynamic_obstacle_logging),
        );
        app.add_systems(PostUpdate, (log_planned_path, log_dynamic_obstacles));
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
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

#[derive(Debug, Clone, Resource)]
pub struct StepPlanner {
    target: Option<Target>,
    reached_translation_target: bool,
    reached_rotation_target: bool,

    static_obstacles: Vec<Obstacle>,
    dynamic_obstacles: Vec<DynamicObstacle>,
}

impl Default for StepPlanner {
    fn default() -> Self {
        StepPlanner {
            target: None,
            reached_translation_target: false,
            reached_rotation_target: false,
            static_obstacles: vec![
                Obstacle::new(4.500, 1.1, 0.2),
                Obstacle::new(4.500, -1.1, 0.2),
                Obstacle::new(-4.500, 1.1, 0.2),
                Obstacle::new(-4.500, -1.1, 0.2),
            ],
            dynamic_obstacles: vec![],
        }
    }
}

impl StepPlanner {
    pub fn set_absolute_target(&mut self, target: Target) {
        self.target = Some(target);
        self.reached_translation_target = false;
        self.reached_rotation_target = false;
    }

    pub fn set_absolute_target_if_unset(&mut self, target: Target) {
        if self.target.is_none() {
            self.set_absolute_target(target);
        }
    }

    pub fn clear_target(&mut self) {
        self.target = None;
        self.reached_translation_target = false;
        self.reached_rotation_target = false;
    }

    #[must_use]
    pub fn current_absolute_target(&self) -> Option<&Target> {
        self.target.as_ref()
    }

    pub fn add_dynamic_obstacle(&mut self, obstacle: DynamicObstacle, merge_distance: f32) {
        match self
            .dynamic_obstacles
            .iter_mut()
            .find(|o| o.obs.distance(&obstacle.obs) <= merge_distance)
        {
            Some(o) => o.ttl = obstacle.ttl,
            None => self.dynamic_obstacles.push(obstacle),
        }
    }

    fn collect_and_gc_dynamic_obstacles(&mut self) -> Vec<Obstacle> {
        let now = Instant::now();

        self.dynamic_obstacles.retain(|obs| now < obs.ttl);
        self.dynamic_obstacles.iter().map(|obs| obs.obs).collect()
    }

    fn get_all_obstacles(&mut self) -> Vec<Obstacle> {
        let mut all_obstacles = self.static_obstacles.clone();
        all_obstacles.extend_from_slice(&self.collect_and_gc_dynamic_obstacles());

        all_obstacles
    }

    fn calc_path(&mut self, robot_pose: &RobotPose) -> Option<(Vec<Point2<f32>>, f32)> {
        let target_position = self.target?.position;
        let all_obstacles = self.get_all_obstacles();

        let abs_obstacles: Vec<_> = all_obstacles
            .iter()
            .map(|obs| {
                let abs_pos = robot_pose.robot_to_world(&Point2::new(obs.x.0, obs.y.0));

                Obstacle::new(abs_pos.x, abs_pos.y, obs.radius.0)
            })
            .collect();

        path_finding::find_path(robot_pose.world_position(), target_position, &abs_obstacles)
    }

    fn plan_translation(robot_pose: &RobotPose, path: &[Point2<f32>]) -> Option<Step> {
        let first_target_position = path[1];
        let distance = calc_distance(&robot_pose.inner, first_target_position);

        // We've reached the target.
        if distance < 0.05 && path.len() == 2 {
            return None;
        }

        let angle = calc_angle_to_point(&robot_pose.inner, first_target_position);
        let turn = calc_turn(&robot_pose.inner, first_target_position);

        if angle > 0.5 {
            Some(Step {
                forward: 0.,
                left: 0.,
                turn,
            })
        } else {
            Some(Step {
                forward: WALK_SPEED,
                left: 0.,
                turn,
            })
        }
    }

    fn plan_rotation(robot_pose: &RobotPose, target_rotation: UnitComplex<f32>) -> Option<Step> {
        let angle = target_rotation.angle() - robot_pose.world_rotation();
        let turn = TURN_SPEED * angle.signum();

        if angle.abs() < 0.2 {
            None
        } else {
            Some(Step {
                forward: 0.,
                left: 0.,
                turn,
            })
        }
    }

    pub fn plan(&mut self, robot_pose: &RobotPose) -> Option<Step> {
        let target = self.target?;
        let (path, _total_walking_distance) = self.calc_path(robot_pose)?;

        if let step @ Some(_) = Self::plan_translation(robot_pose, &path) {
            if !self.reached_translation_target {
                return step;
            }
        }

        self.reached_translation_target = true;

        if let Some(rotation) = target.rotation.as_ref() {
            if let step @ Some(_) = Self::plan_rotation(robot_pose, *rotation) {
                return step;
            }
        }

        self.reached_rotation_target = true;

        None
    }

    #[must_use]
    pub fn reached_target(&self) -> bool {
        self.reached_translation_target && self.reached_rotation_target
    }

    #[must_use]
    pub fn has_target(&self) -> bool {
        self.target.is_some()
    }
}

#[derive(Debug, Clone)]
pub struct DynamicObstacle {
    pub obs: Obstacle,
    pub ttl: Instant,
}

fn calc_turn(pose: &Isometry<f32, UnitComplex<f32>, 2>, target_point: Point2<f32>) -> f32 {
    let relative_transformed_target_point = pose.inverse_transform_point(&target_point);

    relative_transformed_target_point.y.signum() * TURN_SPEED
}

fn calc_angle_to_point(
    pose: &Isometry<f32, UnitComplex<f32>, 2>,
    target_point: Point2<f32>,
) -> f32 {
    let relative_transformed_target_point = pose.inverse_transform_point(&target_point);

    let relative_transformed_target_vector = Vector2::new(
        relative_transformed_target_point.x,
        relative_transformed_target_point.y,
    );

    relative_transformed_target_vector.angle(&Vector2::new(100., 0.))
}

fn calc_distance(pose: &Isometry<f32, UnitComplex<f32>, 2>, target_point: Point2<f32>) -> f32 {
    fn distance(point1: Point2<f32>, point2: Point2<f32>) -> f32 {
        ((point1.x - point2.x).powi(2) + (point1.y - point2.y).powi(2)).sqrt()
    }

    let robot_point = pose.translation.vector.into();

    distance(robot_point, target_point)
}

fn setup_path_visualizer(dbg: DebugContext) {
    dbg.log_with_cycle(
        "field/path",
        Cycle::default(),
        &rerun::LineStrips3D::update_fields()
            .with_colors([(66, 135, 245)])
            .with_radii([2.0]),
    );
}

fn log_planned_path(
    dbg: DebugContext,
    cycle: Res<Cycle>,
    robot_pose: Res<RobotPose>,
    mut step_planner: ResMut<StepPlanner>,
) {
    let path = step_planner.calc_path(&robot_pose);

    if let Some((path, _)) = path {
        dbg.log_with_cycle(
            "field/path",
            *cycle,
            &rerun::LineStrips3D::update_fields().with_strips([LineStrip3D::from_iter(
                path.iter().map(|point| (point.x, point.y, 0.05)),
            )]),
        );
    } else {
        dbg.log_with_cycle(
            "field/path",
            *cycle,
            &rerun::LineStrips3D::update_fields().with_strips(std::iter::empty::<LineStrip3D>()),
        );
    }
}

fn setup_dynamic_obstacle_logging(dbg: DebugContext) {
    dbg.log_static(
        "localization/pose/obstacles",
        &rerun::Ellipsoids3D::update_fields()
            .with_colors([(69, 255, 249)])
            .with_fill_mode(FillMode::Solid),
    );
}

fn log_dynamic_obstacles(dbg: DebugContext, step_planner: Res<StepPlanner>, cycle: Res<Cycle>) {
    let centers = step_planner
        .dynamic_obstacles
        .iter()
        .map(|obs| (obs.obs.x.0, obs.obs.y.0, 0.0))
        .collect::<Vec<_>>();

    let half_sizes = step_planner
        .dynamic_obstacles
        .iter()
        .map(|obs| (obs.obs.radius.0, obs.obs.radius.0, -0.28))
        .collect::<Vec<_>>();

    dbg.log_with_cycle(
        "localization/pose/obstacles",
        *cycle,
        &rerun::Ellipsoids3D::update_fields()
            .with_centers(centers)
            .with_half_sizes(half_sizes),
    );
}
