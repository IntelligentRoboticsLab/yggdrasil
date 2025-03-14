use std::time::Instant;

use bevy::prelude::*;
use heimdall::{Bottom, Top};
use nalgebra::{Normed, Point2, Point3, UnitComplex};
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
            .insert_resource(StrikerWalkStart(None));
    }
}

#[derive(Resource, Deref)]
pub struct StrikerWalkStart(pub Option<Instant>);

/// Substates for the `Striker` role
#[derive(Resource, Default, Debug)]
pub enum Striker {
    #[default]
    WalkToBall,
    WalkAlign,
    WalkWithBall,
}

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
    mut striker_walk_start: ResMut<StrikerWalkStart>,
    mut state: ResMut<Striker>,
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

        let ball_aligned = ball_angle.abs() < 0.2;
        let ball_goal_aligned = absolute_ball_angle < absolute_goal_angle_left
            && absolute_ball_angle > absolute_goal_angle_right;

        let ball_goal_center_align = (absolute_ball_angle - absolute_goal_angle).abs() < 0.2;

        let ball_distance = pose.distance_to(&ball);

        let ball_pos = Target {
            position: ball,
            rotation: None,
        };

        let can_walk_with_ball = can_walk_with_ball(pose.as_ref(), layout_config.as_ref());
        state.next_state(
            can_walk_with_ball,
            ball_distance,
            ball_goal_center_align,
            ball_aligned,
        );

        let relative_ball = most_confident_relative_ball.unwrap();
        let relative_ball_distance2 = relative_ball.norm();
        let relative_ball_distance = f32::sqrt(relative_ball.x.pow(2) + relative_ball.y.pow(2));

        info!(
            ?ball_distance,
            ?relative_ball_distance,
            ?relative_ball_distance2
        );

        // commands.set_behavior(Stand);
        // return;

        // info!(?state, ?ball_distance, ?can_walk_with_ball, ?ball_angle,);

        match *state {
            Striker::WalkToBall => {
                commands.set_behavior(WalkTo { target: ball_pos });
            }
            Striker::WalkAlign => {
                let ball_target = Point3::new(ball.x, ball.y, 0.0);

                if absolute_ball_angle > absolute_goal_angle_left {
                    commands.set_behavior(Walk {
                        step: Step {
                            forward: 0.01,
                            left: 0.08,
                            turn: -0.25,
                        },
                        look_target: Some(ball_target),
                    });
                    return;
                }
                if absolute_ball_angle < absolute_goal_angle_right {
                    commands.set_behavior(Walk {
                        step: Step {
                            forward: 0.01,
                            left: -0.08,
                            turn: 0.25,
                        },
                        look_target: Some(ball_target),
                    });
                }
            }
            Striker::WalkWithBall => {
                // walk with ball for a certain amount of seconds
                if let Some(start) = striker_walk_start.0 {
                    if start.elapsed().as_secs() <= 3 {
                        commands.set_behavior(RlStrikerSearchBehavior);
                        return;
                    } else {
                        striker_walk_start.0 = None;
                    }
                } else {
                    striker_walk_start.0 = Some(Instant::now());
                }
            }
        }
    } else {
    }

    // else if pose.inner.translation.x.abs() > 7.0 && pose.inner.translation.y.abs() > 6.0 {
    //     commands.set_behavior(WalkToSet);
    // } else {
    //     commands.set_behavior(RlStrikerSearchBehavior);
    // }
}

impl Striker {
    fn next_state(
        &mut self,
        can_walk_with_ball: bool,
        ball_distance: f32,
        ball_goal_center_align: bool,
        ball_aligned: bool,
    ) {
        *self = match self {
            _ if ball_distance > BALL_DISTANCE_WALK_THRESHOLD => Striker::WalkToBall,
            Striker::WalkToBall if ball_distance < 0.4 => Striker::WalkAlign,
            Striker::WalkAlign if ball_goal_center_align && ball_aligned => Striker::WalkWithBall,
            Striker::WalkWithBall if can_walk_with_ball => Striker::WalkAlign,
            _ => return,
        }
    }
}

pub fn can_walk_with_ball(pose: &RobotPose, layout_config: &LayoutConfig) -> bool {
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
