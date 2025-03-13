use bevy::prelude::*;

use crate::{
    behavior::{
        behaviors::{Observe, WalkTo},
        engine::{in_role, BehaviorState, CommandsBehaviorExt, RoleState, Roles},
    },
    core::config::{layout::LayoutConfig, showtime::PlayerConfig},
    motion::path::PathPlanner,
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
    planner: Res<PathPlanner>,
    player_config: Res<PlayerConfig>,
    layout_config: Res<LayoutConfig>,
    behavior: Res<State<BehaviorState>>,
) {
    let set_robot_position = layout_config
        .set_positions
        .player(player_config.player_number);

    if planner.reached_target() {
        if behavior.get() != &BehaviorState::Observe {
            commands.set_behavior(Observe::with_turning(0.4));
        }
    } else {
        commands.set_behavior(WalkTo {
            target: set_robot_position.isometry.into(),
        });
    }
}
