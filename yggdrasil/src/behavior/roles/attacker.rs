use nalgebra::{Point2, UnitComplex};

use crate::{
    behavior::{
        behaviors::{Walk, WalkTo},
        engine::{BehaviorKind, Context, Control, Role},
    },
    motion::{step_planner::Target, walk::engine::Step},
};

#[derive(Debug)]
pub struct Attacker;

impl Role for Attacker {
    fn transition_behavior(&mut self, context: Context, control: &mut Control) -> BehaviorKind {
        if let Some(ball) = context.ball_position {
            let enemy_goal_center = Point2::new(context.layout_config.field.length / 2., 0.);
            let enemy_goal_left = Point2::new(context.layout_config.field.length / 2., 0.8);
            let enemy_goal_right = Point2::new(context.layout_config.field.length / 2., -0.8);

            let absolute_goal_angle =
                context.pose.angle_to(&enemy_goal_center) + context.pose.world_rotation();
            let absolute_goal_angle_left =
                context.pose.angle_to(&enemy_goal_left) + context.pose.world_rotation();
            let absolute_goal_angle_right =
                context.pose.angle_to(&enemy_goal_right) + context.pose.world_rotation();

            let ball_angle = context.pose.angle_to(ball);
            let absolute_ball_angle = ball_angle + context.pose.world_rotation();

            let aligned_with_ball = absolute_ball_angle < absolute_goal_angle_left
                && absolute_ball_angle > absolute_goal_angle_right;

            if aligned_with_ball && control.step_planner.reached_target() {
                return BehaviorKind::WalkTo(WalkTo {
                    target: Target {
                        position: *ball,
                        rotation: None,
                    },
                });
            }
            if context.pose.distance_to(ball) < 0.4 {
                control.step_planner.clear_target();
                return BehaviorKind::Walk(Walk {
                    step: Step {
                        left: ball_angle.signum() * 0.04,
                        turn: ball_angle.signum() * 0.4,
                        ..Default::default()
                    },
                });
            } else if context.pose.distance_to(ball) < 1.0 {
                return BehaviorKind::WalkTo(WalkTo {
                    target: Target {
                        position: ball + (ball - enemy_goal_center).normalize() * 0.3,
                        rotation: Some(UnitComplex::from_angle(absolute_goal_angle)),
                    },
                });
            } else {
                return BehaviorKind::WalkTo(WalkTo {
                    target: Target {
                        position: *ball,
                        rotation: None,
                    },
                });
            }
        }
        BehaviorKind::WalkTo(WalkTo {
            target: Target {
                position: context.ball_position.unwrap_or(Point2::new(0.0, 0.0)),
                rotation: None,
            },
        })
    }
}
