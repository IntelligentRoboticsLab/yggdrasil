use std::{f32::NAN, process::exit};

use crate::{
    behavior::{engine::BehaviorKind, Engine},
    filter::orientation::RobotOrientation,
    prelude::*,
    walk::engine::{Step, WalkingEngine},
};

use nalgebra::{Point2, Rotation2, Translation, Vector2};

use super::odometry::Odometry;

pub struct WalkPlannerModule;

impl Module for WalkPlannerModule {
    fn initialize(self, app: App) -> Result<App> {
        let target = WalkPlannerTarget {
            target_position: Point2::new(0., 1.),
        };

        app.add_system(walk_planner_system)
            .add_resource(Resource::new(target))
    }
}

pub struct WalkPlannerTarget {
    target_position: Point2<f32>,
}

fn distance(point1: &Point2<f32>, point2: &Point2<f32>) -> f32 {
    ((point1.x - point2.x).powi(2) + (point1.y - point2.y).powi(2)).sqrt()
}

fn calc_turn(
    robot_translation: &Translation<f32, 2>,
    robot_angle: f32,
    target_point: &Point2<f32>,
) -> f32 {
    let translated_target = robot_translation.transform_point(target_point);

    let rotation = Rotation2::new(-robot_angle);
    let rotated_translated_target = rotation * translated_target;

    eprintln!("rotated_translated_target: {rotated_translated_target}");

    if rotated_translated_target.y < 0. {
        -0.2
    } else {
        0.2
    }
}

#[system]
fn walk_planner_system(
    odometry: &mut Odometry,
    robot_rotation: &RobotOrientation,
    walk_planner_target: &WalkPlannerTarget,
    walking_engine: &mut WalkingEngine,
    behavior_engine: &Engine,
) -> Result<()> {
    if !matches!(behavior_engine.behavior, BehaviorKind::Test(_)) {
        return Ok(());
    }

    // let point = odometry
    //     .accumulated
    //     .rotation
    //     .transform_point(&Point2::new(0.1, 0.0));
    // let point = Vector2::new(point.x, point.y);
    // eprintln!("point: {point}");

    let target_vector = Vector2::new(
        walk_planner_target.target_position.x,
        walk_planner_target.target_position.y,
    );

    let target_vector = odometry.accumulated.transform_vector(&target_vector);

    // eprintln!("target vector: {target_vector}");
    // eprintln!("translation: {}", odometry.accumulated.translation);

    let angle = odometry
        .accumulated
        .inverse()
        .transform_vector(&Vector2::new(0.1, 0.0))
        .angle(&target_vector);
    if angle == NAN {
        exit(0);
    }

    // eprintln!("angle: {}", angle / (2. * std::f32::consts::PI) * 360.);
    // eprintln!("angle: {angle}");

    let robot_point: Point2<f32> = Point2::new(
        odometry.accumulated.translation.x,
        odometry.accumulated.translation.y,
    );

    let target_point = Point2::new(
        walk_planner_target.target_position.x,
        walk_planner_target.target_position.y,
    );

    // eprintln!("target: {target_vector}");
    // eprintln!("point:  {robot_point}");
    // let distance = (target_point - robot_point).norm();
    let distance = distance(&target_point, &robot_point);

    eprintln!("distance: {distance}");
    eprintln!("angle: {angle}");

    if distance < 0.1 {
        walking_engine.request_idle();
    } else if angle > 0.1 {
        let turn = calc_turn(
            &odometry.accumulated.translation,
            robot_rotation.yaw().angle(),
            &walk_planner_target.target_position,
        );
        eprintln!("turn: {turn}");

        walking_engine.request_walk(Step {
            forward: 0.,
            left: 0.,
            turn,
        });
    } else {
        eprintln!("FORWARD");
        walking_engine.request_walk(Step {
            forward: 0.03,
            left: 0.,
            turn: 0.,
        });
    }

    Ok(())
}
