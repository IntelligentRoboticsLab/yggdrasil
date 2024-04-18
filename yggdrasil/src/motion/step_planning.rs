use crate::{
    config::layout::{LayoutConfig, RobotPosition},
    debug::DebugContext,
    prelude::*,
    walk::engine::{Step, WalkingEngine},
};

use nalgebra::{Isometry, Isometry2, Point2, Translation2, Unit, UnitComplex, Vector2};
use nidhogg::types::color;
use num::Complex;

use super::{
    odometry::Odometry,
    path_finding::{self, Obstacle},
};

const TURN_SPEED: f32 = 0.2;
const WALK_SPEED: f32 = 0.03;

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

        app.add_system(walk_planner_system)
            .add_resource(Resource::new(target))
    }
}

pub struct StepPlanner {
    target_position: Option<Point2<f32>>,

    static_obstacles: Vec<Obstacle>,
    dynamic_obstacles: Vec<Obstacle>,
}

impl StepPlanner {
    pub fn set_target(&mut self, target: Point2<f32>) {
        self.target_position = Some(target);
    }

    pub fn set_target_if_unset(&mut self, target: Point2<f32>) {
        if self.target_position.is_none() {
            self.target_position = Some(target);
        }
    }

    pub fn current_target(&self) -> Option<&Point2<f32>> {
        self.target_position.as_ref()
    }

    pub fn set_dynamic_obstacles(&mut self, obstacles: Vec<Obstacle>) {
        self.dynamic_obstacles = obstacles;
    }

    fn get_all_obstacles(&self) -> Vec<Obstacle> {
        let mut all_obstacles = self.static_obstacles.clone();
        all_obstacles.copy_from_slice(&self.dynamic_obstacles);

        all_obstacles
    }
}

fn calc_turn(
    robot_odometry: &Isometry<f32, Unit<Complex<f32>>, 2>,
    target_position: &Point2<f32>,
) -> f32 {
    let translated_target_position = robot_odometry
        .translation
        .inverse_transform_point(target_position);
    let transformed_target_position = robot_odometry
        .rotation
        .inverse_transform_point(&translated_target_position);

    transformed_target_position.y.signum() * TURN_SPEED
}

fn calc_angle(
    robot_odometry: &Isometry<f32, Unit<Complex<f32>>, 2>,
    target_point: &Point2<f32>,
) -> f32 {
    let relative_transformed_target_point = robot_odometry.rotation.inverse_transform_point(
        &robot_odometry
            .translation
            .inverse_transform_point(target_point),
    );

    let relative_transformed_target_vector = Vector2::new(
        relative_transformed_target_point.x,
        relative_transformed_target_point.y,
    );

    relative_transformed_target_vector.angle(&Vector2::new(100., 0.))
}

fn calc_distance(
    robot_odometry: &Isometry<f32, Unit<Complex<f32>>, 2>,
    target_point: &Point2<f32>,
) -> f32 {
    fn distance(point1: &Point2<f32>, point2: &Point2<f32>) -> f32 {
        ((point1.x - point2.x).powi(2) + (point1.y - point2.y).powi(2)).sqrt()
    }
    let robot_point = robot_odometry.transform_point(&Point2::new(0., 0.));

    distance(&robot_point, target_point)
}

fn isometry_to_absolute(
    mut isometry: Isometry2<f32>,
    robot_position: &RobotPosition,
) -> Isometry2<f32> {
    isometry.append_translation_mut(&Translation2::new(
        robot_position.x as f32 / 1000.,
        robot_position.y as f32 / 1000.,
    ));

    isometry.append_rotation_wrt_center_mut(&UnitComplex::from_angle(
        robot_position.rotation.to_radians(),
    ));

    isometry
}

#[system]
fn walk_planner_system(
    odometry: &mut Odometry,
    step_planner: &StepPlanner,
    walking_engine: &mut WalkingEngine,
    dbg: &DebugContext,
    layout_config: &LayoutConfig,
) -> Result<()> {
    let Some(target_position) = step_planner.target_position else {
        return Ok(());
    };
    let all_obstacles = step_planner.get_all_obstacles();

    let Some((path, _total_walking_distance)) = path_finding::find_path(
        odometry.accumulated.transform_point(&Point2::new(0., 0.)),
        target_position,
        &all_obstacles,
    ) else {
        return Ok(());
    };

    let log_path_points: Vec<_> = path.iter().map(|point| (point.x, point.y, 0.)).collect();
    dbg.log_points_3d_with_color_and_radius(
        "/odometry/target",
        &log_path_points,
        color::u8::ORANGE,
        0.04,
    )?;

    // TODO:
    let player_num = 5;
    let isometry = isometry_to_absolute(
        odometry.accumulated,
        layout_config.initial_positions.player(player_num),
    );

    let first_target_position = path[0];
    let turn = calc_turn(&isometry, &first_target_position);
    let angle = calc_angle(&isometry, &first_target_position);
    let distance = calc_distance(&isometry, &first_target_position);

    if distance < 0.1 {
        walking_engine.request_stand();
    } else if angle > 0.3 {
        walking_engine.request_walk(Step {
            forward: 0.,
            left: 0.,
            turn,
        });
    } else {
        walking_engine.request_walk(Step {
            forward: WALK_SPEED,
            left: 0.,
            turn,
        });
    }

    Ok(())
}
