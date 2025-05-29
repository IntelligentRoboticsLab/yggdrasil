use bevy::prelude::*;
use nalgebra::{Point2, Point3};
use nidhogg::types::{FillExt, RightEye, color};

use crate::{
    behavior::{
        behaviors::{RlStrikerSearchBehavior, Walk, WalkTo, WalkToBall},
        engine::{CommandsBehaviorExt, RoleState, Roles, in_role},
    },
    core::config::layout::{FieldConfig, LayoutConfig},
    localization::RobotPose,
    motion::{step_planner::Target, walking_engine::step::Step},
    nao::{NaoManager, Priority},
    vision::ball_detection::ball_tracker::BallTracker,
};

const WALK_WITH_BALL_ANGLE: f32 = 0.3;
const ALIGN_WITH_BALL_DISTANCE: f32 = 0.1;

/// Plugin for the Striker role
pub struct StrikerRolePlugin;

impl Plugin for StrikerRolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, striker_role.run_if(in_role::<Striker>));
    }
}

/// Substates for the `Striker` role
#[derive(Resource, Default, Debug)]
pub struct Striker;

impl Roles for Striker {
    const STATE: RoleState = RoleState::Striker;
}

pub fn striker_role(
    mut commands: Commands,
    pose: Res<RobotPose>,
    layout_config: Res<LayoutConfig>,
    ball_tracker: Res<BallTracker>,
    mut nao_manager: ResMut<NaoManager>,
) {
    let Some(relative_ball) = ball_tracker.stationary_ball() else {
        commands.set_behavior(RlStrikerSearchBehavior);
        return;
    };

    let absolute_ball = pose.robot_to_world(&relative_ball);
    let ball_angle = pose.angle_to(&absolute_ball);
    let ball_distance = relative_ball.coords.norm();
    let ball_target = Point3::new(absolute_ball.x, absolute_ball.y, 0.2);

    let relative_goalpost_left =
        pose.world_to_robot(&Point2::new(layout_config.field.length / 2., 0.8));
    let relative_goalpost_right =
        pose.world_to_robot(&Point2::new(layout_config.field.length / 2., -0.8));

    let goal_aligned = goal_aligned(pose.as_ref(), &layout_config.as_ref().field);

    if ball_distance > ALIGN_WITH_BALL_DISTANCE {
        nao_manager.set_right_eye_led(RightEye::fill(color::f32::YELLOW), Priority::default());

        commands.set_behavior(WalkToBall);
    } else if !goal_aligned {
        nao_manager.set_right_eye_led(RightEye::fill(color::f32::ORANGE), Priority::default());
        if relative_goalpost_left.y < 0. && relative_goalpost_right.y < 0. {
            commands.set_behavior(Walk {
                step: Step {
                    forward: 0.00,
                    left: 0.06,
                    turn: -0.3,
                },
                look_target: Some(ball_target),
            });
            return;
        }
        if relative_goalpost_left.y > 0. && relative_goalpost_right.y > 0. {
            commands.set_behavior(Walk {
                step: Step {
                    forward: 0.00,
                    left: -0.06,
                    turn: 0.3,
                },
                look_target: Some(ball_target),
            });
            return;
        }
    } else if ball_angle.abs() > WALK_WITH_BALL_ANGLE {
        nao_manager.set_right_eye_led(RightEye::fill(color::f32::PURPLE), Priority::default());

        if relative_ball.y.is_sign_negative() {
            commands.set_behavior(Walk {
                step: Step::RIGHT,
                look_target: Some(ball_target),
            });
        } else {
            commands.set_behavior(Walk {
                step: Step::LEFT,
                look_target: Some(ball_target),
            });
        }
    } else {
        nao_manager.set_right_eye_led(RightEye::fill(color::f32::RED), Priority::default());

        commands.set_behavior(Walk {
            step: Step::FORWARD,
            look_target: Some(ball_target),
        });
    }
}

pub fn goal_aligned(pose: &RobotPose, field_config: &FieldConfig) -> bool {
    if pose.inner.translation.x > 0.0 {
        // If on enemy side
        is_aligned_with_goal(pose, field_config)
    } else {
        // If on own side
        is_aligned_with_enemyside(pose, field_config)
    }
}

/// Returns true if we are angled inbetween the goal posts
pub fn is_aligned_with_goal(pose: &RobotPose, field_config: &FieldConfig) -> bool {
    let enemy_goal_left = Point2::new(field_config.length / 2., 0.8);
    let enemy_goal_right = Point2::new(field_config.length / 2., -0.8);

    let relative_goalpost_left = pose.world_to_robot(&enemy_goal_left);
    let relative_goalpost_right = pose.world_to_robot(&enemy_goal_right);

    if relative_goalpost_left.y > 0. && relative_goalpost_right.y < 0. {
        return true;
    }
    false
}

/// Returns true if we are angled inbetween the two corners of the enemy side
pub fn is_aligned_with_enemyside(pose: &RobotPose, field_config: &FieldConfig) -> bool {
    let enemy_goal_left = Point2::new(field_config.length / 2., field_config.width / 2.);
    let enemy_goal_right = Point2::new(field_config.length / 2., -field_config.width / 2.);

    let relative_goalpost_left = pose.world_to_robot(&enemy_goal_left);
    let relative_goalpost_right = pose.world_to_robot(&enemy_goal_right);

    if relative_goalpost_left.y > 0. && relative_goalpost_right.y < 0. {
        return true;
    }
    false
}
