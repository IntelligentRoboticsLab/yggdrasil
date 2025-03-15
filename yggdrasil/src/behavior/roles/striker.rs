use bevy::prelude::*;
use nalgebra::{Point2, Point3};
use nidhogg::types::{color, FillExt, RightEye};

use crate::{
    behavior::{
        behaviors::{RlStrikerSearchBehavior, Walk, WalkToBall},
        engine::{in_role, CommandsBehaviorExt, RoleState, Roles},
    },
    core::config::layout::LayoutConfig,
    localization::RobotPose,
    motion::walking_engine::step::Step,
    nao::{NaoManager, Priority},
    vision::ball_detection::ball_tracker::BallTracker,
};

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
    let Some(ball) = ball_tracker.get_stationary_ball() else {
        commands.set_behavior(RlStrikerSearchBehavior);
        return;
    };

    let relative_ball = pose.world_to_robot(&ball);
    let ball_angle = pose.angle_to(&ball);
    let ball_distance = pose.distance_to(&ball);

    let relative_goalpost_left =
        pose.world_to_robot(&Point2::new(layout_config.field.length / 2., 0.8));
    let relative_goalpost_right =
        pose.world_to_robot(&Point2::new(layout_config.field.length / 2., -0.8));

    let goal_aligned = goal_aligned(pose.as_ref(), layout_config.as_ref());

    let ball_target = Point3::new(ball.x, ball.y, 0.1);
    if ball_distance > 0.3 {
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
    } else if ball_angle.abs() > 0.3 {
        nao_manager.set_right_eye_led(RightEye::fill(color::f32::PURPLE), Priority::default());
        let ball_target = Point3::new(ball.x, ball.y, 0.1);
        if relative_ball.y < 0. {
            // step right
            commands.set_behavior(Walk {
                step: Step {
                    forward: 0.00,
                    left: -0.06,
                    turn: 0.0,
                },
                look_target: Some(ball_target),
            });
        } else {
            // step left
            commands.set_behavior(Walk {
                step: Step {
                    forward: 0.00,
                    left: 0.06,
                    turn: 0.0,
                },
                look_target: Some(ball_target),
            });
        }
    } else {
        nao_manager.set_right_eye_led(RightEye::fill(color::f32::RED), Priority::default());

        commands.set_behavior(Walk {
            step: Step {
                forward: 0.06,
                left: 0.00,
                turn: 0.0,
            },
            look_target: Some(ball_target),
        });
    }
}

pub fn goal_aligned(pose: &RobotPose, layout_config: &LayoutConfig) -> bool {
    if pose.inner.translation.x > 0.0 {
        // If on enemy side
        is_aligned_with_goal(pose, layout_config)
    } else {
        // If on own side
        is_aligned_with_enemyside(pose, layout_config)
    }
}

/// Returns true if we are angled inbetween the goal posts
pub fn is_aligned_with_goal(pose: &RobotPose, layout_config: &LayoutConfig) -> bool {
    let enemy_goal_left = Point2::new(layout_config.field.length / 2., 0.8);
    let enemy_goal_right = Point2::new(layout_config.field.length / 2., -0.8);

    let relative_goalpost_left = pose.world_to_robot(&enemy_goal_left);
    let relative_goalpost_right = pose.world_to_robot(&enemy_goal_right);

    if relative_goalpost_left.y > 0. && relative_goalpost_right.y < 0. {
        return true;
    }
    false
}

/// Returns true if we are angled inbetween the two corners of the enemy side
pub fn is_aligned_with_enemyside(pose: &RobotPose, layout_config: &LayoutConfig) -> bool {
    let enemy_goal_left = Point2::new(
        layout_config.field.length / 2.,
        layout_config.field.width / 2.,
    );
    let enemy_goal_right = Point2::new(
        layout_config.field.length / 2.,
        -layout_config.field.width / 2.,
    );

    let relative_goalpost_left = pose.world_to_robot(&enemy_goal_left);
    let relative_goalpost_right = pose.world_to_robot(&enemy_goal_right);

    if relative_goalpost_left.y > 0. && relative_goalpost_right.y < 0. {
        return true;
    }
    false
}
