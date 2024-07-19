use nalgebra::{ComplexField, Point2, UnitComplex};

use crate::{
    behavior::{
        behaviors::{Observe, Walk, WalkTo},
        engine::{BehaviorKind, Context, Control, Role},
    },
    motion::{step_planner::Target, walk::engine::Step},
};

#[derive(Debug, Default, Clone, Copy)]
pub enum ScoreGoalStates {
    #[default]
    WalkToBall,
    WalkToAlignPos(Target),
    WalkAlign,
    WalkWithBall,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Attacker {
    state: ScoreGoalStates,
}

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

            let ball_aligned = ball_angle.abs() < 0.2;
            let ball_goal_aligned = absolute_ball_angle < absolute_goal_angle_left
                && absolute_ball_angle > absolute_goal_angle_right;

            let ball_goal_center_align = (absolute_ball_angle - absolute_goal_angle).abs() < 0.2;

            let ball_distance = context.pose.distance_to(ball);

            let ball_pos = Target {
                position: *ball,
                rotation: None,
            };

            self.state = match &self.state {
                _ if ball_distance > 0.5 => ScoreGoalStates::WalkToBall,

                ScoreGoalStates::WalkToBall if ball_distance < 0.3 => ScoreGoalStates::WalkAlign,
                ScoreGoalStates::WalkAlign if ball_goal_center_align && ball_aligned => {
                    ScoreGoalStates::WalkWithBall
                }
                ScoreGoalStates::WalkWithBall if !ball_goal_aligned => ScoreGoalStates::WalkAlign,
                _ => self.state.clone(),
            };
            match self.state {
                ScoreGoalStates::WalkToBall => {
                    return BehaviorKind::WalkTo(WalkTo { target: ball_pos });
                }
                ScoreGoalStates::WalkToAlignPos(align_pos) => {
                    return BehaviorKind::WalkTo(WalkTo { target: align_pos });
                }
                ScoreGoalStates::WalkAlign => {
                    if absolute_ball_angle > absolute_goal_angle_left {
                        return BehaviorKind::Walk(Walk {
                            step: Step {
                                left: 0.03,
                                turn: -0.33,
                                ..Default::default()
                            },
                        });
                    }
                    if absolute_ball_angle < absolute_goal_angle_right {
                        return BehaviorKind::Walk(Walk {
                            step: Step {
                                left: -0.03,
                                turn: 0.33,
                                ..Default::default()
                            },
                        });
                    }
                }
                ScoreGoalStates::WalkWithBall => {
                    return BehaviorKind::WalkTo(WalkTo { target: ball_pos });
                }
            }
        }

        if context.pose.distance_to(&Point2::origin()) < 0.2 {
            if let BehaviorKind::Observe(observe) = context.current_behavior {
                return BehaviorKind::Observe(observe);
            } else {
                return BehaviorKind::Observe(Observe::with_turning(0.4));
            };
        }
        BehaviorKind::WalkTo(WalkTo {
            target: Target {
                position: context.ball_position.unwrap_or(Point2::new(0.0, 0.0)),
                rotation: None,
            },
        })
    }
}
