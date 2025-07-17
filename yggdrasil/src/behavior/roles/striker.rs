use bevy::prelude::*;
use bifrost::communication::{GameControllerMessage, SetPlay};
use nalgebra::{Point2, Point3};
use nidhogg::types::{FillExt, RightEye, color};

use crate::{
    behavior::{
        behaviors::{
            LookMode, LostBallSearch, RlStrikerSearchBehavior, StandLookAt, Walk, WalkTo,
            WalkToBall,
        },
        engine::{BehaviorState, CommandsBehaviorExt, RoleState, Roles, in_role},
        primary_state::PrimaryState,
    },
    core::config::{
        layout::{FieldConfig, LayoutConfig},
        showtime::PlayerConfig,
    },
    localization::RobotPose,
    motion::{step_planner::Target, walking_engine::step::Step},
    nao::{NaoManager, Priority},
    vision::ball_detection::TeamBallPosition,
};

use std::time::Duration;
use crate::vision::ball_detection::ball_tracker::BallTracker;

const WALK_WITH_BALL_ANGLE: f32 = 0.3;
const ALIGN_WITH_BALL_DISTANCE: f32 = 0.3;

/// Plugin for the Striker role
pub struct StrikerRolePlugin;

impl Plugin for StrikerRolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                striker_role.run_if(in_role::<Striker>.and(not(in_set_play))),
                set_play.run_if(in_set_play),
            ),
        )
        .add_systems(OnExit(RoleState::Striker), reset_striker_role);
    }
}

fn in_set_play(
    gamecontroller_message: Option<Res<GameControllerMessage>>,
    primary_state: Res<PrimaryState>,
    player_config: Res<PlayerConfig>,
) -> bool {
    if let Some(message) = gamecontroller_message {
        return match *primary_state {
            // return true if there is a set play OR we are in Playing state with a secondary time (Kick-Off)
            PrimaryState::Playing { .. } => {
                (message.set_play != SetPlay::None || message.secondary_time != 0)
                    && message.kicking_team != player_config.team_number
            }
            _ => false,
        };
    }

    false
}

/// The `Striker` role has five substates, each indicated by the right eye LED color:
///
/// | LED Color | Substate            | Description                                                      |
/// |-----------|---------------------|------------------------------------------------------------------|
/// | Green     | RL Striker Search   | No ball detected; search for the ball.                           |
/// | Yellow    | Walk to Ball        | Ball is far; walk straight towards it.                           |
/// | Orange    | Align with Goal     | Close to the ball but not aligned with the goal; circle step.    |
/// | Purple    | Align with Ball     | Aligned with the goal but not with the ball; side step to align. |
/// | Red       | Walk with Ball      | Aligned with both goal and ball; walk straight forward.          |
#[derive(Resource, Default, Debug)]
pub struct Striker;

#[derive(Resource)]
pub struct LostBallSearchTimer {
    timer: Timer,
    last_ball: Point2<f32>,
}

impl LostBallSearchTimer {
    #[must_use]
    pub fn new(duration: Duration, last_ball: Point2<f32>) -> Self {
        LostBallSearchTimer {
            timer: Timer::new(duration, TimerMode::Once),
            last_ball,
        }
    }
}

impl Roles for Striker {
    const STATE: RoleState = RoleState::Striker;
}

fn reset_striker_role(mut nao_manager: ResMut<NaoManager>) {
    nao_manager.set_right_eye_led(RightEye::fill(color::f32::EMPTY), Priority::default());
}

pub fn striker_role(
    mut commands: Commands,
    pose: Res<RobotPose>,
    layout_config: Res<LayoutConfig>,
    detected_ball_position: Res<TeamBallPosition>,
    mut nao_manager: ResMut<NaoManager>,
    lost_ball_timer: Option<ResMut<LostBallSearchTimer>>,
    time: Res<Time>,
) {
    let Some(relative_ball) = detected_ball_position.0 else {
        if let Some(mut timer) = lost_ball_timer {
            timer.timer.tick(time.delta()); // <- tick the timer

            if timer.timer.finished() {
                commands.remove_resource::<LostBallSearchTimer>();
            } else {
                nao_manager
                    .set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::default()); // TEMP LED

                // determine the side we need to turn to by using timer.last_ball
                let relative_last_ball = &timer.last_ball;
                commands.set_behavior(LostBallSearch::with_turning(
                    relative_last_ball.y.signum() * 0.6, //TODO test
                ));
            }
        } else {
            nao_manager.set_right_eye_led(RightEye::fill(color::f32::GREEN), Priority::default());
            commands.set_behavior(RlStrikerSearchBehavior);
        }
        return;
    };
    let absolute_ball: nalgebra::OPoint<f32, nalgebra::Const<2>> =
        pose.robot_to_world(&relative_ball);

    if ball_tracker.timestamp.elapsed().as_secs_f32() > 0.5 {
        if lost_ball_timer.is_none() {
            commands.insert_resource(LostBallSearchTimer::new(
                Duration::from_secs(9),
                relative_ball,
            ));
        }
    } else {
        commands.remove_resource::<LostBallSearchTimer>();
    }

    let absolute_ball = pose.robot_to_world(&relative_ball);
    let ball_angle = pose.angle_to(&absolute_ball);
    let ball_distance = relative_ball.coords.norm();
    let ball_target: nalgebra::OPoint<f32, nalgebra::Const<3>> =
        Point3::new(absolute_ball.x, absolute_ball.y, 0.2);

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

//TODO: Make this a separate stand-alone behavior
fn set_play(
    mut commands: Commands,
    ball_tracker: Res<BallTracker>,
    pose: Res<RobotPose>,
    behavior_state: Res<State<BehaviorState>>,
    walk: Option<Res<Walk>>,
) {
    let Some(relative_ball) = ball_tracker.stationary_ball() else {
        return;
    };
    let absolute_ball = pose.robot_to_world(&relative_ball);
    let ball_distance = relative_ball.coords.norm();
    let ball_target = Point3::new(absolute_ball.x, absolute_ball.y, 0.2);

    if ball_distance > 1.2 {
        commands.set_behavior(WalkTo {
            target: Target {
                position: absolute_ball,
                rotation: None,
            },
            look_mode: LookMode::AtTarget,
        });
        return;
    }

    if behavior_state.get() == &BehaviorState::Walk {
        if let Some(walk) = walk {
            if matches!(walk.step, Step::BACK) && ball_distance < 0.875 {
                return;
            }
        }
    }

    if ball_distance < 0.75 {
        commands.set_behavior(Walk {
            step: Step::BACK,
            look_target: Some(ball_target),
        });
        return;
    }

    commands.set_behavior(StandLookAt {
        target: absolute_ball,
    });
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
