use crate::{localization::RobotPose, motion::walk::engine::Step, prelude::*};

use nalgebra::{Isometry, Point2, Unit, Vector2};
use num::Complex;

use super::path_finding::{self, Obstacle};

const TURN_SPEED: f32 = 0.8;
const WALK_SPEED: f32 = 0.05;

pub struct StepPlannerModule;

impl Module for StepPlannerModule {
    fn initialize(self, app: App) -> Result<App> {
        let target = StepPlanner {
            target_position: None,
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
    target_position: Option<Point2<f32>>,

    static_obstacles: Vec<Obstacle>,
    dynamic_obstacles: Vec<Obstacle>,
}

impl StepPlanner {
    pub fn set_absolute_target(&mut self, target: Point2<f32>) {
        self.target_position = Some(target);
    }

    pub fn set_absolute_target_if_unset(&mut self, target: Point2<f32>) {
        if self.target_position.is_none() {
            self.target_position = Some(target);
        }
    }

    pub fn current_absolute_target(&self) -> Option<&Point2<f32>> {
        self.target_position.as_ref()
    }

    pub fn set_dynamic_obstacles(&mut self, obstacles: Vec<Obstacle>) {
        self.dynamic_obstacles = obstacles;
    }

    fn get_all_obstacles(&self) -> Vec<Obstacle> {
        let mut all_obstacles = self.static_obstacles.clone();
        all_obstacles.extend_from_slice(&self.dynamic_obstacles);

        all_obstacles
    }

    pub fn plan(&mut self, current_pose: &RobotPose) -> Option<Step> {
        let target_position = self.target_position?;
        let all_obstacles = self.get_all_obstacles();

        let (path, _total_walking_distance) = path_finding::find_path(
            current_pose.world_position(),
            target_position,
            &all_obstacles,
        )?;
        // Not sure if this is possible, needs more testing, but it will prevent a panic later on.
        if path.len() == 1 {
            return None;
        }

        let first_target_position = path[1];
        let turn = calc_turn(&current_pose.inner, &first_target_position);
        let angle = calc_angle(&current_pose.inner, &first_target_position);
        let distance = calc_distance(&current_pose.inner, &first_target_position);

        if distance < 0.2 && path.len() == 2 {
            None
        } else if angle > 0.3 {
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
}

fn calc_turn(pose: &Isometry<f32, Unit<Complex<f32>>, 2>, target_position: &Point2<f32>) -> f32 {
    let translated_target_position = pose.translation.inverse_transform_point(target_position);
    let transformed_target_position = pose
        .rotation
        .inverse_transform_point(&translated_target_position);

    transformed_target_position.y.signum() * TURN_SPEED
}

fn calc_angle(pose: &Isometry<f32, Unit<Complex<f32>>, 2>, target_point: &Point2<f32>) -> f32 {
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
