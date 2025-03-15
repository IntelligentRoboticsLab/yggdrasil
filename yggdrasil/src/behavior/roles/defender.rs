use bevy::prelude::*;
use heimdall::{Bottom, Top};
use nalgebra::Point2;

use crate::{
    behavior::{
        behaviors::{RlDefenderSearchBehavior, WalkTo},
        engine::{in_role, CommandsBehaviorExt, RoleState, Roles},
        roles::Striker,
    },
    core::config::{layout::LayoutConfig, showtime::PlayerConfig},
    localization::RobotPose,
    motion::path::Target,
    vision::ball_detection::classifier::Balls,
};

const DISTANCE_FROM_TARGET_FINISH: f32 = 0.2;

/// Plugin for the Defender role
pub struct DefenderRolePlugin;

impl Plugin for DefenderRolePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<RobotFieldRegion>()
            .init_state::<WalkToPosition>()
            .add_systems(
                Update,
                (
                    return_walk_progress.before(defender_role),
                    is_outside_field.before(defender_role),
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

/// A bevy state ([`States`]), which keeps track of whether the robot is inside
/// or outside the field
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum RobotFieldRegion {
    #[default]
    Inside,
    Outside,
}

// Checks if the robot is outside or inside the field based on the robots position
// and the field size. This updates the state `RobotFieldRegion`
fn is_outside_field(
    robot_pose: Res<RobotPose>,
    layout_config: Res<LayoutConfig>,
    mut next_field_region: ResMut<NextState<RobotFieldRegion>>,
) {
    let field_length = layout_config.field.length;
    let field_width = layout_config.field.width;

    let robot_position = robot_pose.world_position();

    let outside_horizontal = robot_position.x.abs() > field_length / 2.0;
    let outside_vertical = robot_position.y.abs() > field_width / 2.0;

    if outside_horizontal || outside_vertical {
        next_field_region.set(RobotFieldRegion::Outside);
    } else {
        next_field_region.set(RobotFieldRegion::Inside)
    }
}

#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum WalkToPosition {
    #[default]
    Walking,
    Finished,
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
