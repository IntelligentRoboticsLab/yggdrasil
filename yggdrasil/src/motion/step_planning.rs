use crate::{
    behavior::{engine::BehaviorKind, Engine},
    debug::DebugContext,
    prelude::*,
    walk::engine::{Step, WalkingEngine},
};

use nalgebra::{Isometry, Point2, Unit, Vector2};
use nidhogg::types::color;
use num::Complex;

use super::{
    odometry::Odometry,
    path_finding::{self, Obstacle},
};

const TURN_SPEED: f32 = 0.2;
const WALK_SPEED: f32 = 0.03;

pub struct WalkPlannerModule;

impl Module for WalkPlannerModule {
    fn initialize(self, app: App) -> Result<App> {
        let target = WalkPlannerTarget {
            target_position: Some(Point2::new(3., 0.0)),
        };

        app.add_system(walk_planner_system)
            .add_resource(Resource::new(target))
    }
}

pub struct WalkPlannerTarget {
    target_position: Option<Point2<f32>>,
}

impl WalkPlannerTarget {
    pub fn set(&mut self, target: Point2<f32>) {
        self.target_position = Some(target);
    }

    pub fn set_if_unset(&mut self, target: Point2<f32>) {
        if self.target_position.is_none() {
            self.target_position = Some(target);
        }
    }

    pub fn current_target(&self) -> Option<&Point2<f32>> {
        self.target_position.as_ref()
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

#[system]
fn walk_planner_system(
    odometry: &mut Odometry,
    walk_planner_target: &WalkPlannerTarget,
    walking_engine: &mut WalkingEngine,
    behavior_engine: &Engine,
    dbg: &DebugContext,
) -> Result<()> {
    let Some(target_position) = walk_planner_target.target_position else {
        return Ok(());
    };

    let Some((path, _total_walking_distance)) = path_finding::find_path(
        odometry.accumulated.transform_point(&Point2::new(0., 0.)),
        target_position,
        &[Obstacle::new(1., 0., 0.2)],
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

    if !matches!(behavior_engine.behavior, BehaviorKind::Test(_)) {
        return Ok(());
    }

    let first_target_position = path[0];
    let turn = calc_turn(&odometry.accumulated, &first_target_position);
    let angle = calc_angle(&odometry.accumulated, &first_target_position);
    let distance = calc_distance(&odometry.accumulated, &first_target_position);

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
            forward: WALK_SPEED,
            left: 0.,
            turn,
        });
    }

    Ok(())
}
