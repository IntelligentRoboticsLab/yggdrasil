use bevy::prelude::*;
use nalgebra::Point2;

use crate::{
    behavior::{
        behaviors::{Observe, WalkTo},
        engine::{in_role, CommandsBehaviorExt, RoleState, Roles},
    },
    core::config::{layout::LayoutConfig, showtime::PlayerConfig},
    motion::step_planner::{StepPlanner, Target},
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
    player_config: Res<PlayerConfig>,
    layout_config: Res<LayoutConfig>,
    step_planner: ResMut<StepPlanner>,
) {
    let set_robot_position = layout_config
        .set_positions
        .player(player_config.player_number);
    let set_position = set_robot_position.isometry.translation.vector;
    let set_point = Point2::new(set_position.x, set_position.y);
    let defend_target = Target {
        position: set_point,
        rotation: Some(set_robot_position.isometry.rotation),
    };

    if !step_planner.has_target() {
        commands.set_behavior(WalkTo {
            target: defend_target,
        });
        return;
    }

    if step_planner.reached_target() {
        commands.set_behavior(Observe::with_turning(-0.4));
    } else {
        commands.set_behavior(WalkTo {
            target: defend_target,
        });
    }
}
