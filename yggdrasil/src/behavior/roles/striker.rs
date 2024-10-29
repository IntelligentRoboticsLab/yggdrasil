use nalgebra::{Point2, Point3};

use crate::{
    behavior::{
        behaviors::{Observe, Walk, WalkTo},
        engine::{BehaviorKind, Context, Control, Role},
    },
    localization::RobotPose,
    motion::{step_planner::Target, walk::engine::Step},
};

/// The [`Striker`] role is held by a robot when it is can see the ball.
/// It contains three substates for walking to the ball, aligning with the ball and the goal, and walking with the ball whilst aligned.
#[derive(Debug, Default, Clone, Copy)]
pub enum Striker {
    #[default]
    WalkToBall,
    WalkAlign,
    WalkWithBall,
}

impl Role for Striker {
    fn transition_behavior(&mut self, context: Context, _control: &mut Control) -> BehaviorKind {
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

            let ball_aligned = ball_angle.abs() < 0.2;
            let ball_goal_aligned = absolute_ball_angle < absolute_goal_angle_left
                && absolute_ball_angle > absolute_goal_angle_right;

            let ball_goal_center_align = (absolute_ball_angle - absolute_goal_angle).abs() < 0.2;

            let ball_distance = context.pose.distance_to(ball);

            let ball_pos = Target {
                position: *ball,
                rotation: None,
            };

            *self = self.next_state(
                ball_distance,
                ball_goal_center_align,
                ball_aligned,
                ball_goal_aligned,
            );
            match self {
                Striker::WalkToBall | Striker::WalkWithBall => {
                    return BehaviorKind::WalkTo(WalkTo { target: ball_pos });
                }
                Striker::WalkAlign => {
                    let ball_target = Point3::new(ball.x, ball.y, RobotPose::CAMERA_HEIGHT);

                    if absolute_ball_angle > absolute_goal_angle_left {
                        return BehaviorKind::Walk(Walk {
                            step: Step {
                                left: 0.03,
                                turn: -0.3,
                                ..Default::default()
                            },
                            look_target: Some(ball_target),
                        });
                    }
                    if absolute_ball_angle < absolute_goal_angle_right {
                        return BehaviorKind::Walk(Walk {
                            step: Step {
                                left: -0.03,
                                turn: 0.3,
                                ..Default::default()
                            },
                            look_target: Some(ball_target),
                        });
                    }
                }
            }
        }

        if context.pose.distance_to(&Point2::origin()) < 0.2 {
            if let BehaviorKind::Observe(observe) = context.current_behavior {
                return BehaviorKind::Observe(observe);
            }

            return BehaviorKind::Observe(Observe::with_turning(0.4));
        }

        BehaviorKind::WalkTo(WalkTo {
            target: Target {
                position: context.ball_position.unwrap_or(Point2::new(0.0, 0.0)),
                rotation: None,
            },
        })
    }
}

impl Striker {
    fn next_state(
        self,
        ball_distance: f32,
        ball_goal_center_align: bool,
        ball_aligned: bool,
        ball_goal_aligned: bool,
    ) -> Striker {
        match self {
            _ if ball_distance > 0.5 => Striker::WalkToBall,
            Striker::WalkToBall if ball_distance < 0.3 => Striker::WalkAlign,
            Striker::WalkAlign if ball_goal_center_align && ball_aligned => Striker::WalkWithBall,
            Striker::WalkWithBall if !ball_goal_aligned => Striker::WalkAlign,

            _ => self,
        }
    }
}
