use bevy::prelude::*;
use heimdall::{Bottom, Top};
use nalgebra::{Point2, Point3};

use crate::{
    behavior::{
        behaviors::{Observe, Walk, WalkTo},
        engine::{in_role, BehaviorState, CommandsBehaviorExt, Role, Roles},
    },
    core::config::layout::LayoutConfig,
    localization::RobotPose,
    motion::{step_planner::Target, walking_engine::step::Step},
    vision::ball_detection::classifier::Balls,
};

// Walk to the ball as long as the ball is further away than
// `BALL_DISTANCE_WALK_THRESHOLD` meters.
const BALL_DISTANCE_WALK_THRESHOLD: f32 = 0.5;

/// Plugin for the Striker role
pub struct StrikerRolePlugin;

impl Plugin for StrikerRolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, striker_role.run_if(in_role::<Striker>));
    }
}

/// Substates for the `Striker` role
#[derive(Resource, Default)]
pub enum Striker {
    #[default]
    WalkToBall,
    WalkAlign,
    WalkWithBall,
}

impl Roles for Striker {
    const STATE: Role = Role::Striker;
}

pub fn striker_role(
    mut commands: Commands,
    pose: Res<RobotPose>,
    layout_config: Res<LayoutConfig>,
    top_balls: Res<Balls<Top>>,
    bottom_balls: Res<Balls<Bottom>>,
    mut state: ResMut<Striker>,
    behavior_state: Res<State<BehaviorState>>,
) {
    let most_confident_ball = bottom_balls
        .most_confident_ball()
        .map(|b| b.position)
        .or(top_balls.most_confident_ball().map(|b| b.position));

    if let Some(ball) = most_confident_ball {
        let enemy_goal_center = Point2::new(layout_config.field.length / 2., 0.);
        let enemy_goal_left = Point2::new(layout_config.field.length / 2., 0.8);
        let enemy_goal_right = Point2::new(layout_config.field.length / 2., -0.8);

        let absolute_goal_angle = pose.angle_to(&enemy_goal_center) + pose.world_rotation();
        let absolute_goal_angle_left = pose.angle_to(&enemy_goal_left) + pose.world_rotation();
        let absolute_goal_angle_right = pose.angle_to(&enemy_goal_right) + pose.world_rotation();

        let ball_angle = pose.angle_to(&ball);
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

        state.next_state(
            ball_distance,
            ball_goal_center_align,
            ball_aligned,
            ball_goal_aligned,
        );

        match *state {
            Striker::WalkToBall | Striker::WalkWithBall => {
                commands.set_behavior(WalkTo { target: ball_pos });
            }
            Striker::WalkAlign => {
                let ball_target = Point3::new(ball.x, ball.y, RobotPose::CAMERA_HEIGHT);

                if absolute_ball_angle > absolute_goal_angle_left {
                    commands.set_behavior(Walk {
                        step: Step {
                            left: 0.06,
                            turn: -0.3,
                            ..Default::default()
                        },
                        look_target: Some(ball_target),
                    });
                    return;
                }
                if absolute_ball_angle < absolute_goal_angle_right {
                    commands.set_behavior(Walk {
                        step: Step {
                            left: -0.06,
                            turn: 0.3,
                            ..Default::default()
                        },
                        look_target: Some(ball_target),
                    });
                }
            }
        }
    } else if behavior_state.get() != &BehaviorState::Observe {
        commands.set_behavior(Observe::with_turning(0.4));
    }
}

impl Striker {
    fn next_state(
        &mut self,
        ball_distance: f32,
        ball_goal_center_align: bool,
        ball_aligned: bool,
        ball_goal_aligned: bool,
    ) {
        *self = match self {
            _ if ball_distance > BALL_DISTANCE_WALK_THRESHOLD => Striker::WalkToBall,
            Striker::WalkToBall if ball_distance < 0.3 => Striker::WalkAlign,
            Striker::WalkAlign if ball_goal_center_align && ball_aligned => Striker::WalkWithBall,
            Striker::WalkWithBall if !ball_goal_aligned => Striker::WalkAlign,
            _ => return,
        }
    }
}
