use std::time::Instant;

use bevy::prelude::*;
use heimdall::{Bottom, Top};
use nalgebra::{Normed, Point2, Point3, UnitComplex};
use nidhogg::types::{color, FillExt, RightEye};
use ordered_float::Pow;

use crate::{
    behavior::{
        behaviors::{RlStrikerSearchBehavior, Stand, Walk, WalkTo, WalkToSet},
        engine::{in_role, CommandsBehaviorExt, RoleState, Roles},
    },
    core::config::layout::LayoutConfig,
    localization::RobotPose,
    motion::{
        step_planner::{self, Target},
        walking_engine::step::Step,
    },
    nao::{NaoManager, Priority},
    vision::ball_detection::classifier::Balls,
};

// Walk to the ball as long as the ball is further away than
// `BALL_DISTANCE_WALK_THRESHOLD` meters.
const BALL_DISTANCE_WALK_THRESHOLD: f32 = 0.75;

/// Plugin for the Striker role
pub struct StrikerRolePlugin;

impl Plugin for StrikerRolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, striker_role.run_if(in_role::<Striker>))
            .insert_resource(StrikerWalkStart(None))
            .insert_resource(StrikerState::WalkToBall);
    }
}

#[derive(Resource, Deref)]
pub struct StrikerWalkStart(pub Option<Instant>);

#[derive(Resource, Default, Debug)]
pub enum StrikerState {
    #[default]
    WalkToBall,
    WalkAlign,
    WalkWithBall,
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
    top_balls: Res<Balls<Top>>,
    bottom_balls: Res<Balls<Bottom>>,
    step_planner: ResMut<step_planner::StepPlanner>,
    mut nao_manager: ResMut<NaoManager>,
    mut striker_walk_start: ResMut<StrikerWalkStart>,
    mut state: ResMut<StrikerState>,
) {
    let most_confident_ball = bottom_balls
        .most_confident_ball()
        .map(|b| b.position)
        .or(top_balls.most_confident_ball().map(|b| b.position));

    let most_confident_relative_ball = bottom_balls
        .most_confident_ball()
        .map(|b| b.robot_to_ball)
        .or(top_balls.most_confident_ball().map(|b| b.robot_to_ball));

    if let Some(ball) = most_confident_ball {
        let relative_ball = most_confident_relative_ball.expect("This would be a funny error");
        let enemy_goal_center = Point2::new(layout_config.field.length / 2., 0.);
        let enemy_goal_left = Point2::new(layout_config.field.length / 2., 0.8);
        let enemy_goal_right = Point2::new(layout_config.field.length / 2., -0.8);

        let absolute_goal_angle = pose.angle_to(&enemy_goal_center) + pose.world_rotation();
        let absolute_goal_angle_left = pose.angle_to(&enemy_goal_left) + pose.world_rotation();
        let absolute_goal_angle_right = pose.angle_to(&enemy_goal_right) + pose.world_rotation();

        let ball_angle = pose.angle_to(&ball);
        let relative_ball_angle = relative_ball.y.atan2(relative_ball.x);
        let absolute_ball_angle = ball_angle + pose.world_rotation();

        let enemy_goal_left = Point2::new(layout_config.field.length / 2., 0.8);
        let enemy_goal_right = Point2::new(layout_config.field.length / 2., -0.8);

        let relative_goal_left = pose.world_to_robot(&enemy_goal_left);
        let relative_goal_right = pose.world_to_robot(&enemy_goal_right);

        let ball_aligned = ball_angle.abs() < 0.2;
        let ball_goal_aligned = absolute_ball_angle < absolute_goal_angle_left
            && absolute_ball_angle > absolute_goal_angle_right;

        let ball_goal_center_align = (absolute_ball_angle - absolute_goal_angle).abs() < 0.2;

        let ball_distance = pose.distance_to(&ball);

        let ball_pos = Target {
            position: ball,
            rotation: None,
        };

        let goal_aligned = goal_aligned(pose.as_ref(), layout_config.as_ref());
        // state.next_state(
        //     goal_aligned,
        //     ball_distance,
        //     ball_goal_center_align,
        //     ball_aligned,
        // );
        let relative_ball = pose.world_to_robot(&ball);

        // let relative_ball = most_confident_relative_ball.unwrap();
        // let relative_ball_distance2 = relative_ball.norm();
        // let relative_ball_distance = f32::sqrt(relative_ball.x.pow(2) + relative_ball.y.pow(2));

        // info!(
        //     ?ball_distance,
        //     ?relative_ball_distance,
        //     ?relative_ball_distance2
        // );

        // commands.set_behavior(Stand);
        // return;

        info!(?ball_distance, ?goal_aligned, ?ball_angle,);

        let ball_target = Point3::new(ball.x, ball.y, 0.1);
        if ball_distance > 0.3 {
            nao_manager.set_right_eye_led(RightEye::fill(color::f32::YELLOW), Priority::default());

            commands.set_behavior(WalkTo { target: ball_pos });
        } else {
            if !goal_aligned {
                nao_manager
                    .set_right_eye_led(RightEye::fill(color::f32::ORANGE), Priority::default());
                if relative_goal_left.y < 0. && relative_goal_right.y < 0. {
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
                if relative_goal_left.y > 0. && relative_goal_right.y > 0. {
                    commands.set_behavior(Walk {
                        step: Step {
                            forward: 0.00,
                            left: -0.06,
                            turn: 0.3,
                        },
                        look_target: Some(ball_target),
                    });
                    return;
                } else {
                    println!("WATAAFAAAAK!?!?!?!");
                }
            } else if ball_angle.abs() > 0.3 {
                nao_manager
                    .set_right_eye_led(RightEye::fill(color::f32::PURPLE), Priority::default());
                let ball_target = Point3::new(ball.x, ball.y, 0.1);
                if relative_ball.y < 0. {
                    // step right
                    commands.set_behavior(Walk {
                        step: Step {
                            forward: 0.00,
                            left: -0.04,
                            turn: 0.0,
                        },
                        look_target: Some(ball_target),
                    });
                } else {
                    // step left
                    commands.set_behavior(Walk {
                        step: Step {
                            forward: 0.00,
                            left: 0.04,
                            turn: 0.0,
                        },
                        look_target: Some(ball_target),
                    });
                }
                println!("We Were goal algined, but not ball aligned!!!!");

                // walk to align with ball
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

                // if let Some(start) = striker_walk_start.0 {
                //     if start.elapsed().as_secs() <= 3 {
                //         commands.set_behavior(RlStrikerSearchBehavior);
                //         return;
                //     } else {
                //         striker_walk_start.0 = None;
                //     }
                // } else {
                //     striker_walk_start.0 = Some(Instant::now());
                // }
            }
        }
    }

    // commands.set_behavior(Stand);
    // return;

    // Aligned
    // left post is on the left    right post is on the right
    // relative_goal_left.y > 0. && relative_goal_right.y < 0.

    // everything is to the right
    // if relative_goal_left.y <0 && relative_goal_right.y < 0

    // everything is to the left
    // if relative_goal_left.y > 0 && relative_goal_right.y > 0

    //     println!("Komen we hier?!?!?!?!");
    // }
    // StrikerState::WalkWithBall => {

    //     // walk with ball for a certain amount of seconds

    // }
    //     }
    // } else {
    // }

    // else if pose.inner.translation.x.abs() > 7.0 && pose.inner.translation.y.abs() > 6.0 {
    //     commands.set_behavior(WalkToSet);
    // } else {
    //     commands.set_behavior(RlStrikerSearchBehavior);
    // }
}

impl StrikerState {
    fn next_state(
        &mut self,
        goal_aligned: bool,
        ball_distance: f32,
        ball_goal_center_align: bool,
        ball_aligned: bool,
    ) {
        *self = match self {
            _ if ball_distance > BALL_DISTANCE_WALK_THRESHOLD => StrikerState::WalkToBall,
            StrikerState::WalkAlign if goal_aligned && ball_aligned => StrikerState::WalkWithBall,
            StrikerState::WalkWithBall if !goal_aligned => StrikerState::WalkAlign,
            _ => return,
        }
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

pub fn is_aligned_with_goal(pose: &RobotPose, layout_config: &LayoutConfig) -> bool {
    // returns true if we are angled inbetween the goal posts

    let enemy_goal_left = Point2::new(layout_config.field.length / 2., 0.8);
    let enemy_goal_right = Point2::new(layout_config.field.length / 2., -0.8);

    let relative_goal_left = pose.world_to_robot(&enemy_goal_left);
    let relative_goal_right = pose.world_to_robot(&enemy_goal_right);

    if relative_goal_left.y > 0. && relative_goal_right.y < 0. {
        return true;
    } else {
        return false;
    }
}

// returns true if we are angled inbetween the two corners of the enemy side
pub fn is_aligned_with_enemyside(pose: &RobotPose, layout_config: &LayoutConfig) -> bool {
    let enemy_goal_left = Point2::new(
        layout_config.field.length / 2.,
        layout_config.field.width / 2.,
    );
    let enemy_goal_right = Point2::new(
        layout_config.field.length / 2.,
        -layout_config.field.width / 2.,
    );

    let relative_goal_left = pose.world_to_robot(&enemy_goal_left);
    let relative_goal_right = pose.world_to_robot(&enemy_goal_right);

    if relative_goal_left.y > 0. && relative_goal_right.y < 0. {
        return true;
    } else {
        return false;
    }
}
