use bevy::prelude::*;
use heimdall::{Bottom, Top};
use nalgebra::Point2;

use crate::{
    behavior::{
        behaviors::{RlDefenderSearchBehavior, WalkTo},
        engine::{in_role, CommandsBehaviorExt, RoleState, Roles, WalkToPosition},
        roles::Striker,
    },
    core::config::{layout::LayoutConfig, showtime::PlayerConfig},
    localization::{RobotFieldRegion, RobotPose},
    motion::path::Target,
    vision::ball_detection::classifier::Balls,
};

const DISTANCE_FROM_TARGET_FINISH: f32 = 0.2;

/// Plugin for the Defender role
pub struct DefenderRolePlugin;

impl Plugin for DefenderRolePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
                Update,
                (
                    return_walk_progress.before(defender_role),
                    defender_role.run_if(in_role::<Defender>),
                ),
            );
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
    robot_field_region: Res<State<RobotFieldRegion>>,
    return_walk: Res<State<WalkToPosition>>,
    layout_config: Res<LayoutConfig>,
    player_config: Res<PlayerConfig>,
) {
    let most_confident_ball = bottom_balls
        .most_confident_ball()
        .map(|b| b.position)
        .or(top_balls.most_confident_ball().map(|b| b.position));

    let set_robot_position = layout_config
        .set_positions
        .player(player_config.player_number);

    if most_confident_ball.is_none() {
        match (robot_field_region.get(), return_walk.get()) {
            (RobotFieldRegion::Inside, WalkToPosition::Finished) => {
                commands.set_behavior(RlDefenderSearchBehavior);
            }

            (RobotFieldRegion::Outside, ..)
            | (RobotFieldRegion::Inside, WalkToPosition::Walking) => {
                commands.set_behavior(WalkTo {
                    target: Target::Isometry(set_robot_position.isometry),
                });
            }
        }
    } else {
        commands.set_role(Striker::WalkToBall);
    }
}

fn return_walk_progress(
    mut walk_progress: ResMut<NextState<WalkToPosition>>,
    robot_pose: Res<RobotPose>,
    layout_config: Res<LayoutConfig>,
    player_config: Res<PlayerConfig>,
) {
    let set_robot_position = layout_config
        .set_positions
        .player(player_config.player_number);

    let target_point = Point2::from(set_robot_position.isometry.translation.vector);
    let distance_to_target = robot_pose.distance_to(&target_point);

    if distance_to_target < DISTANCE_FROM_TARGET_FINISH {
        walk_progress.set(WalkToPosition::Walking);
    } else {
        walk_progress.set(WalkToPosition::Finished);
    }
}
