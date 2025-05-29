use std::time::{Duration, Instant};

use crate::{
    core::config::layout::{FieldConfig, LayoutConfig},
    motion::{path_finding::Obstacle, step_planner::DynamicObstacle},
};
use bevy::prelude::*;
use nalgebra::{Point2, Point3, UnitComplex, Vector2, Vector3};

use crate::{
    behavior::engine::{Behavior, BehaviorState, in_behavior},
    localization::RobotPose,
    motion::{
        step_planner::{StepPlanner, Target},
        walking_engine::step_context::StepContext,
    },
    nao::{NaoManager, Priority},
    vision::ball_detection::ball_tracker::BallTracker,
};

const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);
/// How far behind the ball we stand before beginning the dribble.
const DRIBBLE_START_OFFSET: f32 = 0.2;

pub struct WalkToBallBehaviorPlugin;

impl Plugin for WalkToBallBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, walk_to_ball.run_if(in_behavior::<WalkToBall>));
    }
}

#[derive(Resource)]
pub struct WalkToBall;

impl Behavior for WalkToBall {
    const STATE: BehaviorState = BehaviorState::WalkToBall;
}

fn walk_to_ball(
    pose: Res<RobotPose>,
    mut step_planner: ResMut<StepPlanner>,
    mut step_context: ResMut<StepContext>,
    mut nao_manager: ResMut<NaoManager>,
    ball_tracker: Res<BallTracker>,
    layout: Res<LayoutConfig>,
) {
    // 1) get ball in world frame
    let relative_ball = match ball_tracker.stationary_ball() {
        Some(rel) => rel,
        None => return,
    };
    let ball_world = pose.robot_to_world(&relative_ball);

    // 3) compute the behind‐the‐ball dribble‐start pose
    let (target_pos, target_yaw) =
        compute_dribble_start_pose(&ball_world, &layout.field, DRIBBLE_START_OFFSET);

    let dribble_target = if relative_ball.coords.magnitude() <= 0.3 {
        Target {
            position: ball_world,
            rotation: Some(UnitComplex::from_angle(target_yaw)),
        }
    } else {
        step_planner.add_dynamic_obstacle(
            DynamicObstacle {
                obs: Obstacle::new(relative_ball.x, relative_ball.y, 0.1),
                ttl: Instant::now() + Duration::from_millis(50),
            },
            0.1,
        );

        Target {
            position: target_pos,
            rotation: Some(UnitComplex::from_angle(target_yaw)),
        }
    };

    // 2) always keep head on the real ball
    let look_at = pose.get_look_at_absolute(&Point3::new(ball_world.x, ball_world.y, 0.0));
    nao_manager.set_head_target(
        look_at,
        HEAD_ROTATION_TIME,
        Priority::default(),
        NaoManager::HEAD_STIFFNESS,
    );

    // 4) if the existing target differs, clear it
    if step_planner.current_absolute_target().is_some_and(|t| {
        t.position != dribble_target.position || t.rotation != dribble_target.rotation
    }) {
        step_planner.clear_target();
    }

    // 5) set our new target
    step_planner.set_absolute_target_if_unset(dribble_target);

    // 6) plan & execute
    if let Some(step) = step_planner.plan(&pose) {
        step_context.request_walk(step);
    } else {
        step_context.request_stand();
    }
}

/// Compute a “dribble start” pose: a point behind the ball,
/// facing the opponent’s goal, ready to take control and drive.
///
/// # Arguments
/// * `ball_world` –_*

/// Compute a “dribble start” pose: a point behind the ball,
/// facing the opponent’s goal, ready to take control and drive.
///
/// # Arguments
/// * `ball_world` – the ball’s current position in world‐frame (x,y,z).
/// * `field`      – your FieldConfig (for goal location).
/// * `offset`     – how far behind the ball to stand (in meters).
///
/// # Returns
/// * `(target_pos, yaw)`  
///    – `target_pos`: Point3<f32> where the robot should step to,  
///    – `yaw`:        f32 with the heading in radians so +X is 0, CCW positive,
///                   oriented toward the opponent’s goal.
pub fn compute_dribble_start_pose(
    ball_world: &Point2<f32>,
    field: &FieldConfig,
    offset: f32,
) -> (Point2<f32>, f32) {
    // Opponent’s goal centre in world coords (assume z = ball z)
    let goal_world = Point2::new(field.length / 2.0, 0.0);

    // Vector from goal to ball
    let to_ball: Vector2<f32> = ball_world - goal_world;
    let dir = to_ball.normalize();

    // Place the robot “offset” meters behind the ball along that line
    let target_coords = ball_world.coords + dir * offset;
    let target_pos = Point2::new(target_coords.x, target_coords.y);

    // Yaw so robot faces goal: vector from target to goal
    let look_vec = goal_world - target_pos;
    let yaw = look_vec.y.atan2(look_vec.x);

    (target_pos, yaw)
}
