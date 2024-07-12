use crate::{localization::RobotPose, motion::walk::engine::Step, prelude::*};

use nalgebra::{Isometry, Isometry2, Point2, Unit, Vector2};
use num::Complex;

use super::path_finding::{self, Obstacle};

const TURN_SPEED: f32 = 0.8;
const WALK_SPEED: f32 = 0.05;
const LEFT_SPEED: f32 = 0.03;

pub struct StepPlannerModule;

impl Module for StepPlannerModule {
    fn initialize(self, app: App) -> Result<App> {
        let target = StepPlanner {
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
        };

        app.add_resource(Resource::new(target))
    }
}

pub struct StepPlanner {
    target: Option<Isometry2<f32>>,
    reached_translation_target: bool,
    reached_rotation_target: bool,

    static_obstacles: Vec<Obstacle>,
    dynamic_obstacles: Vec<Obstacle>,
}

impl StepPlanner {
    pub fn set_absolute_target(&mut self, target: Isometry2<f32>) {
        self.target = Some(target);
        self.reached_translation_target = false;
        self.reached_rotation_target = false;
    }

    pub fn set_absolute_target_if_unset(&mut self, target: Isometry2<f32>) {
        if self.target.is_none() {
            self.set_absolute_target(target);
        }
    }

    pub fn clear_target(&mut self) {
        self.target = None;
        self.reached_translation_target = false;
        self.reached_rotation_target = false;
    }

    pub fn current_absolute_target(&self) -> Option<&Isometry2<f32>> {
        self.target.as_ref()
    }

    pub fn set_dynamic_obstacles(&mut self, obstacles: Vec<Obstacle>) {
        self.dynamic_obstacles = obstacles;
    }

    fn get_all_obstacles(&self) -> Vec<Obstacle> {
        let mut all_obstacles = self.static_obstacles.clone();
        all_obstacles.extend_from_slice(&self.dynamic_obstacles);

        all_obstacles
    }

    fn calc_path(&self, robot_pose: &RobotPose) -> Option<(Vec<Point2<f32>>, f32)> {
        let target_position = self
            .target
            .map(|isometry| isometry.translation.transform_point(&Point2::new(0.0, 0.0)))?;
        let all_obstacles = self.get_all_obstacles();

        path_finding::find_path(robot_pose.world_position(), target_position, &all_obstacles)
    }

    fn plan_translation(&mut self, robot_pose: &RobotPose, path: &[Point2<f32>]) -> Option<Step> {
        let first_target_position = path[1];
        let distance = calc_distance(&robot_pose.inner, &first_target_position);
        if distance < 0.2 && path.len() == 2 {
            return None;
        }

        let angle = calc_angle_to_point(&robot_pose.inner, &first_target_position);
        let turn = calc_turn(&robot_pose.inner, &first_target_position, angle);

        if angle > 0.5 {
            // let left = if turn > 0. { LEFT_SPEED } else { -LEFT_SPEED };
            let left = 0.;

            Some(Step {
                forward: 0.,
                left,
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

    fn plan_rotation(
        &self,
        robot_pose: &RobotPose,
        target_rotation: &Unit<Complex<f32>>,
    ) -> Option<Step> {
        let angle = target_rotation.angle() - robot_pose.world_rotation();
        let turn = TURN_SPEED * angle.signum();

        // TODO: This is currently necessary because, according to odometry, the robot walks around
        // when it turns around its axis.
        // Once that is fixed (with localization using line detection), this early return should
        // probably be removed.
        if angle.abs() <= 0.4 {
            return None;
        }

        Some(Step {
            forward: 0.,
            left: 0.,
            turn,
        })
    }

    pub fn plan(&mut self, robot_pose: &RobotPose) -> Option<Step> {
        let target = self.target?;
        let (path, _total_walking_distance) = self.calc_path(robot_pose)?;

        if let step @ Some(_) = self.plan_translation(robot_pose, &path) {
            if !self.reached_translation_target {
                return step;
            }
        }

        self.reached_translation_target = true;

        if let step @ Some(_) = self.plan_rotation(robot_pose, &target.rotation) {
            return step;
        }

        self.reached_rotation_target = true;

        None
    }
}

fn calc_turn(
    pose: &Isometry<f32, Unit<Complex<f32>>, 2>,
    target_position: &Point2<f32>,
    angle: f32,
) -> f32 {
    let translated_target_position = pose.translation.inverse_transform_point(target_position);
    let transformed_target_position = pose
        .rotation
        .inverse_transform_point(&translated_target_position);

    // transformed_target_position.y.signum() * TURN_SPEED * (angle / std::f32::consts::FRAC_PI_2)
    transformed_target_position.y.signum() * TURN_SPEED
}

fn calc_angle_to_point(
    pose: &Isometry<f32, Unit<Complex<f32>>, 2>,
    target_point: &Point2<f32>,
) -> f32 {
    let relative_transformed_target_point = pose
        .rotation
        .inverse_transform_point(&pose.translation.inverse_transform_point(target_point));

    let relative_transformed_target_vector = Vector2::new(
        relative_transformed_target_point.x,
        relative_transformed_target_point.y,
    );

    relative_transformed_target_vector.angle(&Vector2::new(100., 0.))
}

fn calc_distance(pose: &Isometry<f32, Unit<Complex<f32>>, 2>, target_point: &Point2<f32>) -> f32 {
    fn distance(point1: &Point2<f32>, point2: &Point2<f32>) -> f32 {
        ((point1.x - point2.x).powi(2) + (point1.y - point2.y).powi(2)).sqrt()
    }
    let robot_point = pose.transform_point(&Point2::new(0., 0.));

    distance(&robot_point, target_point)
}
