use bevy::prelude::*;
use nalgebra::Point3;

use crate::{
    behavior::engine::{Behavior, BehaviorState, in_behavior},
    localization::RobotPose,
    motion::{
        step_planner::{StepPlanner, Target},
        walking_engine::step_context::StepContext,
    },
    nao::{HeadMotionManager, LookAt},
    vision::ball_detection::hypothesis::Ball,
};

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
    mut head_motion_manager: ResMut<HeadMotionManager>,
    ball: Res<Ball>,
) {
    let Some(ball) = ball.position().map(|ball| pose.robot_to_world(&ball)) else {
        return;
    };

    let ball_target = Target::from(ball);
    let target_point = Point3::new(ball.x, ball.y, 0.0);

    head_motion_manager.request_look_at(LookAt {
        pose: *pose,
        point: target_point,
    });

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
