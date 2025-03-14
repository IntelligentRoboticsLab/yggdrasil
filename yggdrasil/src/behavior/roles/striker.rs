use std::time::Duration;

use bevy::prelude::*;
use nalgebra::{Isometry, Point2, Point3, UnitComplex, Vector2};

use crate::{
    behavior::{
        behaviors::{RlStrikerSearchBehavior, Stand, Walk, WalkTo},
        engine::{in_role, CommandsBehaviorExt, RoleState, Roles},
    },
    core::config::layout::LayoutConfig,
    localization::RobotPose,
    motion::path::{geometry::Circle, obstacles::Obstacle, PathPlanner, Target},
    motion::walking_engine::step::Step,
    nao::{NaoManager, Priority},
    vision::ball_detection::{ball_tracker::BallTracker, Hypothesis},
};

// Walk to the ball as long as the ball is further away than
// `BALL_DISTANCE_WALK_THRESHOLD` meters.
const BALL_DISTANCE_WALK_THRESHOLD: f32 = 0.5;

const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);

/// Plugin for the Striker role
pub struct StrikerRolePlugin;

impl Plugin for StrikerRolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, better_striker.run_if(in_role::<Striker>))
            .insert_resource(BallObstacleId(None));
    }
}

#[derive(Resource, Deref)]
pub struct BallObstacleId(pub Option<Entity>);

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

pub fn better_striker(
    mut commands: Commands,
    pose: Res<RobotPose>,
    layout_config: Res<LayoutConfig>,
    ball_tracker: Res<BallTracker>,
    mut state: ResMut<Striker>,
    planner: Res<PathPlanner>,
    mut ball_obstacle: ResMut<BallObstacleId>,
    mut nao_manager: ResMut<NaoManager>,
) {
    if let Hypothesis::Stationary(_) = ball_tracker.cutoff() {
        let ball = ball_tracker.state().0;

        let enemy_goal_center = Point2::new(layout_config.field.length / 2., 0.);

        let current_world_rotation = pose.world_rotation();

        let direction = (enemy_goal_center - ball).normalize();
        let target_point = ball + direction * 0.01;
        let target_rotation = (enemy_goal_center - target_point).angle(&Vector2::new(1.0, 0.0))
            + current_world_rotation;

        let ball_angle = pose.angle_to(&ball);
        let target_reached = planner.reached_target(pose.inner);
        info!(?ball_angle, ?target_reached);

        if target_reached && ball_angle.abs() < 0.3 {
            if let Some(ball_obstacle_id) = ball_obstacle.0 {
                if let Some(mut ball_obstacle_entity) = commands.get_entity(ball_obstacle_id) {
                    ball_obstacle_entity.despawn();
                }
                commands.entity(ball_obstacle_id).despawn();
                ball_obstacle.0 = None;
            }

            info!("Reached target!!!!!");
            commands.set_behavior(Walk {
                step: Step {
                    forward: 0.05,
                    left: 0.0,
                    turn: 0.0,
                },
                look_target: Some(Point3::new(ball.x, ball.y, 0.0)),
            });
        } else {
            if ball_obstacle.0.is_none() {
                let ball_obstacle_id = commands.spawn(Obstacle::from(Circle::new(ball, 0.05))).id();
                ball_obstacle.0 = Some(ball_obstacle_id);
            }

            let look_at = pose.get_look_at_absolute(&nalgebra::point![ball.x, ball.y, 0.1]);

            info!("Walking to obstacle!!!!!");

            nao_manager.set_head_target(
                look_at,
                HEAD_ROTATION_TIME,
                Priority::Medium,
                NaoManager::HEAD_STIFFNESS,
            );

            commands.set_behavior(WalkTo {
                target: Target::Isometry(Isometry::from_parts(
                    target_point.into(),
                    UnitComplex::from_angle(target_rotation),
                )),
            });
        }
        // Use target_point and target_rotation as needed
    } else {
        ball_obstacle.0.map(|id| {
            if let Some(mut ball_obstacle_entity) = commands.get_entity(id) {
                ball_obstacle_entity.despawn();
                ball_obstacle.0 = None;
            }
        });
        commands.set_behavior(Stand);
    }
}

// pub fn striker_role(
//     mut commands: Commands,
//     pose: Res<RobotPose>,
//     layout_config: Res<LayoutConfig>,
//     top_balls: Res<Balls<Top>>,
//     bottom_balls: Res<Balls<Bottom>>,
//     mut state: ResMut<Striker>,
// ) {
//     let most_confident_ball = bottom_balls
//         .most_confident_ball()
//         .map(|b| b.position)
//         .or(top_balls.most_confident_ball().map(|b| b.position));

//     if let Some(ball) = most_confident_ball {
//         let enemy_goal_center = Point2::new(layout_config.field.length / 2., 0.);
//         let enemy_goal_left = Point2::new(layout_config.field.length / 2., 0.8);
//         let enemy_goal_right = Point2::new(layout_config.field.length / 2., -0.8);

//         let absolute_goal_angle = pose.angle_to(&enemy_goal_center) + pose.world_rotation();
//         let absolute_goal_angle_left = pose.angle_to(&enemy_goal_left) + pose.world_rotation();
//         let absolute_goal_angle_right = pose.angle_to(&enemy_goal_right) + pose.world_rotation();

//         let ball_angle = pose.angle_to(&ball);
//         let absolute_ball_angle = ball_angle + pose.world_rotation();

//         let ball_aligned = ball_angle.abs() < 0.2;
//         let ball_goal_aligned = absolute_ball_angle < absolute_goal_angle_left
//             && absolute_ball_angle > absolute_goal_angle_right;

//         let ball_goal_center_align = (absolute_ball_angle - absolute_goal_angle).abs() < 0.2;

//         let ball_distance = pose.distance_to(&ball);

//         state.next_state(
//             ball_distance,
//             ball_goal_center_align,
//             ball_aligned,
//             ball_goal_aligned,
//         );

//         info!(
//             ?ball_goal_center_align,
//             ?ball_goal_aligned,
//             ?ball_aligned,
//             ?ball_angle,
//             ?ball_distance
//         );

//         match *state {
//             Striker::WalkToBall | Striker::WalkWithBall => {
//                 commands.set_behavior(Stand);
//                 info!(
//                     ?ball_goal_center_align,
//                     ?ball_goal_aligned,
//                     ?ball_aligned,
//                     ?ball_angle,
//                     ?ball_distance
//                 );

//                 return;
//                 commands.set_behavior(WalkTo {
//                     target: ball.into(),
//                 });
//             }
//             Striker::WalkAlign => {
//                 let ball_target = Point3::new(ball.x, ball.y, 0.0);

//                 if absolute_ball_angle > absolute_goal_angle_left {
//                     commands.set_behavior(Walk {
//                         step: Step {
//                             forward: 0.01,
//                             left: 0.08,
//                             turn: -0.25,
//                         },
//                         look_target: Some(ball_target),
//                     });
//                     return;
//                 }
//                 if absolute_ball_angle < absolute_goal_angle_right {
//                     commands.set_behavior(Walk {
//                         step: Step {
//                             forward: 0.01,
//                             left: -0.08,
//                             turn: 0.25,
//                         },
//                         look_target: Some(ball_target),
//                     });
//                 }
//             }
//         }
//     } else {
//         commands.set_behavior(RlStrikerSearchBehavior);
//     }
// }

// impl Striker {
//     fn next_state(
//         &mut self,
//         ball_distance: f32,
//         ball_goal_center_align: bool,
//         ball_aligned: bool,
//         ball_goal_aligned: bool,
//     ) {
//         *self = match self {
//             _ if ball_distance > BALL_DISTANCE_WALK_THRESHOLD => Striker::WalkToBall,
//             Striker::WalkToBall if ball_distance < 0.3 => Striker::WalkAlign,
//             Striker::WalkAlign if ball_goal_center_align && ball_aligned => Striker::WalkWithBall,
//             Striker::WalkWithBall if !ball_goal_aligned => Striker::WalkAlign,
//             _ => return,
//         }
//     }
// }
