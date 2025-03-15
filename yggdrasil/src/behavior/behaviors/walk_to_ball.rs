use std::time::Duration;

use bevy::prelude::*;
use nalgebra::Point3;

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState},
    localization::RobotPose,
    motion::{
        step_planner::{StepPlanner, Target},
        walking_engine::step_context::StepContext,
    },
    nao::{NaoManager, Priority},
    vision::ball_detection::ball_tracker::BallTracker,
};

const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);

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

pub fn walk_to_ball(
    pose: Res<RobotPose>,
    mut step_planner: ResMut<StepPlanner>,
    mut step_context: ResMut<StepContext>,
    mut nao_manager: ResMut<NaoManager>,
    ball_tracker: Res<BallTracker>,
) {
    let Some(ball) = ball_tracker.get_stationary_ball() else {
        return;
    };

    let ball_target = Target::from(ball);
    let target_point = Point3::new(ball.x, ball.y, 0.0);

    let look_at = pose.get_look_at_absolute(&target_point);
    nao_manager.set_head_target(
        look_at,
        HEAD_ROTATION_TIME,
        Priority::default(),
        NaoManager::HEAD_STIFFNESS,
    );

    // Check and clear existing target if different
    if step_planner
        .current_absolute_target()
        .is_some_and(|target| {
            target
                != &Target {
                    position: ball,
                    rotation: None,
                }
        })
    {
        step_planner.clear_target();
    }

    // Set absolute target if not set
    step_planner.set_absolute_target_if_unset(ball_target);

    // Plan step or stand
    if let Some(step) = step_planner.plan(&pose) {
        step_context.request_walk(step);
    } else {
        step_context.request_stand();
    }
}
