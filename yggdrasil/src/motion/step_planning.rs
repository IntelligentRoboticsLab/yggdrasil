use crate::{
    behavior::{engine::BehaviorKind, Engine},
    debug::DebugContext,
    prelude::*,
    walk::engine::{Step, WalkingEngine},
};

use nalgebra::{Isometry, Point2, Unit, Vector2};
use nidhogg::types::color;
use num::Complex;

use super::odometry::Odometry;

pub struct WalkPlannerModule;

impl Module for WalkPlannerModule {
    fn initialize(self, app: App) -> Result<App> {
        let target = WalkPlannerTarget {
            target_position: Point2::new(-1., -1.),
        };

        app.add_system(walk_planner_system)
            .add_resource(Resource::new(target))
    }
}

pub struct WalkPlannerTarget {
    target_position: Point2<f32>,
}

fn calc_angle(
    robot_odometry: &Isometry<f32, Unit<Complex<f32>>, 2>,
    target_point: &Point2<f32>,
) -> f32 {
    let target_to_world = target_point;
    let target_to_world_vector = Vector2::new(target_to_world.x, target_to_world.y);

    let robot_to_world = robot_odometry.translation.vector.xy();
    let angle = robot_to_world.angle(&target_to_world_vector);
    angle - robot_odometry.rotation.angle()
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

#[system]
fn walk_planner_system(
    odometry: &mut Odometry,
    walk_planner_target: &WalkPlannerTarget,
    walking_engine: &mut WalkingEngine,
    behavior_engine: &Engine,
    dbg: &DebugContext,
) -> Result<()> {
    if !matches!(behavior_engine.behavior, BehaviorKind::Test(_)) {
        return Ok(());
    }

    let target_to_world = &walk_planner_target.target_position;
    let robot_to_target = odometry.accumulated.transform_point(target_to_world);

    dbg.log_points_3d_with_color_and_radius(
        "/odometry/target",
        &[
            (target_to_world.x, target_to_world.y, 0.),
            (robot_to_target.x, robot_to_target.y, 0.),
        ],
        color::u8::ORANGE,
        0.04,
    )?;

    let angle = calc_angle(&odometry.accumulated, &walk_planner_target.target_position);
    eprintln!("ANGLE: {}", angle);

    let turn = if angle > 0.0 { -0.3 } else { 0.3 };
    eprintln!("TURN:  {turn}");

    let distance = calc_distance(&odometry.accumulated, &walk_planner_target.target_position);
    eprintln!("DISTANCE: {distance}");

    if distance < 0.1 {
        walking_engine.request_idle();
    } else if angle > 0.3 {
        walking_engine.request_walk(Step {
            forward: 0.,
            left: 0.,
            turn,
        });
    } else {
        walking_engine.request_walk(Step {
            forward: 0.05,
            left: 0.,
            turn: 0.0,
        });
    }

    Ok(())
}
