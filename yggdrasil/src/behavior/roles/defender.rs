use bevy::prelude::*;
use heimdall::{Bottom, Top};

use crate::{
    behavior::{
        behaviors::{RlDefenderDribbleBehavior, RlDefenderSearchBehavior, WalkToSet},
        engine::{in_role, CommandsBehaviorExt, RoleState, Roles},
    },
    vision::ball_detection::classifier::Balls,
};

/// Plugin for the Defender role
pub struct DefenderRolePlugin;

impl Plugin for DefenderRolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, defender_role.run_if(in_role::<Defender>));
    }
}

/// The [`Defender`] role is held by any robot that does not see the ball.
/// It's job is to observe it's set position depending on player number.
#[derive(Resource)]
pub struct Defender;

impl Roles for Defender {
    const STATE: RoleState = RoleState::Defender;
}

pub fn defender_role(
    mut commands: Commands,
    top_balls: Res<Balls<Top>>,
    bottom_balls: Res<Balls<Bottom>>,
) {
    let most_confident_ball = bottom_balls
        .most_confident_ball()
        .map(|b| b.position)
        .or(top_balls.most_confident_ball().map(|b| b.position));

    if let Some(ball) = most_confident_ball {
        if ball.x > 0.0 {
            commands.set_behavior(WalkToSet);
        } else {
            commands.set_behavior(RlDefenderDribbleBehavior);
        }
    } else {
        commands.set_behavior(RlDefenderSearchBehavior);
    }
}

// pub fn defender_role(
//     mut commands: Commands,
//     pose: Res<RobotPose>,
//     player_config: Res<PlayerConfig>,
//     layout_config: Res<LayoutConfig>,
//     step_planner: ResMut<StepPlanner>,
// ) {
//     let set_robot_position = layout_config
//         .set_positions
//         .player(player_config.player_number);
//     let set_position = set_robot_position.isometry.translation.vector;
//     let set_point = Point2::new(set_position.x, set_position.y);
//     let defend_target = Target {
//         position: set_point,
//         rotation: Some(set_robot_position.isometry.rotation),
//     };

//     let close_to_target = pose.distance_to(&set_point) < 0.5;
//     let aligned_with_rotation =
//         (pose.world_rotation() - set_robot_position.isometry.rotation.angle()).abs() < 0.2;

//     if step_planner.has_target() && step_planner.reached_target()
//         || (close_to_target && aligned_with_rotation)
//     {
//         commands.set_behavior(Observe::with_turning(-0.4));
//     } else {
//         commands.set_behavior(WalkTo {
//             target: defend_target,
//         });
//     }
// }
