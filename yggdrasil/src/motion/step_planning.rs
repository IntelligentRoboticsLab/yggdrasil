use crate::{
    config::{
        layout::{LayoutConfig, RobotPosition},
        showtime::ShowtimeConfig,
    },
    debug::DebugContext,
    nao::{
        manager::{NaoManager, Priority},
        RobotInfo,
    },
    prelude::*,
    walk::engine::{Step, WalkingEngine},
};

use crate::motion::odometry;

use nalgebra::{Isometry, Point2, Unit, Vector2};
use nidhogg::types::{color, FillExt, HeadJoints};
use num::Complex;

use super::{
    odometry::Odometry,
    path_finding::{self, Obstacle},
};

const TURN_SPEED: f32 = 0.3;
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
        all_obstacles.extend_from_slice(&self.dynamic_obstacles);

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

fn look_at_target(robot_position: &Point2<f32>, target_point: &Point2<f32>) -> HeadJoints<f32> {
    // Transform center point from world space to robot space.
    let sign = robot_position.y.signum() as f32;
    let transformed_center_x = robot_position.x - target_point.x * sign;
    let transformed_center_y = robot_position.y - target_point.y * sign;

    // Compute angle and then convert to the nek yaw, this angle is dependent on
    // which side of the field the robot is located.
    let angle = (transformed_center_y / transformed_center_x).atan();
    let yaw = (std::f32::consts::FRAC_PI_2 + angle * sign) * sign;

    HeadJoints { yaw, pitch: 0.0 }
}

#[system]
fn walk_planner_system(
    odometry: &mut Odometry,
    step_planner: &StepPlanner,
    walking_engine: &mut WalkingEngine,
    layout_config: &LayoutConfig,
    showtime_config: &ShowtimeConfig,
    robot_info: &RobotInfo,
    ctx: &DebugContext,
    nao: &mut NaoManager,
) -> Result<()> {
    if walking_engine.is_sitting() {
        return Ok(());
    }

    let Some(target_position) = step_planner.target_position else {
        return Ok(());
    };
    let all_obstacles = step_planner.get_all_obstacles();

    let Some((path, _total_walking_distance)) = path_finding::find_path(
        odometry.accumulated.translation.vector.into(),
        target_position,
        &all_obstacles,
    ) else {
        return Ok(());
    };
    // Not sure if this is possible, needs more testing, but it will prevent a panic later on.
    if path.len() == 1 {
        return Ok(());
    }

    let player_num = showtime_config.robot_numbers_map[&robot_info.robot_id.to_string()];
    let isometry = odometry::isometry_to_absolute(
        odometry.accumulated,
        layout_config.initial_positions.player(player_num),
    );

    let first_target_position = path[1];
    let turn = calc_turn(&isometry, &first_target_position);
    let angle = calc_angle(&isometry, &first_target_position);
    let distance = calc_distance(&isometry, &first_target_position);

    ctx.log_points_3d_with_color_and_radius(
        "/odometry/target_position",
        &[(target_position.x, target_position.y, 0.0)],
        color::u8::BLUE,
        0.04,
    )?;

    if distance < 0.1 && path.len() == 2 {
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

    // nao.set_head(
    //     look_at_target(&isometry.translation.vector.into(), &first_target_position),
    //     HeadJoints::fill(0.4),
    //     Priority::High,
    // );

    Ok(())
}
