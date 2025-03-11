use bevy::prelude::*;
use bifrost::communication::GameControllerMessage;
use nalgebra::Point2;

use crate::{
    behavior::{
        behaviors::{Observe, WalkTo},
        engine::{in_role, BehaviorState, CommandsBehaviorExt, RoleState, Roles},
    }, core::config::{layout::LayoutConfig, showtime::PlayerConfig}, game_controller, localization::RobotPose, motion::step_planner::{StepPlanner, Target}
};

const GIJS_HAS_KICK_WORKING: bool = false;

/// Plugin for the KickIn role
pub struct KickInRolePlugin;

impl Plugin for KickInRolePlugin {
    fn build(&self, app: &mut App) {
        if GIJS_HAS_KICK_WORKING {
            app.add_systems(Update, kick_in_role.run_if(in_role::<KickIn>));
        } else {
            app.add_systems(Update, kick_walk_role.run_if(in_role::<KickIn>));
        }
    }
}

/// The [`KickIn`] role is temporary held by a signle robot.
/// It's first implementation will stay limited to the kick-in for the start of the game.
#[derive(Resource)]
pub struct KickIn;
impl Roles for KickIn {
    const STATE: RoleState = RoleState::KickIn;
}

pub fn kick_in_role(){
    // TODO: Wait for @oxkitsune to make a kick in the walking engine
}

pub fn kick_walk_role(
    mut commands: Commands,
    pose: Res<RobotPose>,
    player_config: Res<PlayerConfig>,
    layout_config: Res<LayoutConfig>,
    step_planner: ResMut<StepPlanner>,
    behavior: Res<State<BehaviorState>>,
    game_controller_message: Option<Res<GameControllerMessage>>,
) {
    if let Some(game_state) = game_controller_message.as_deref() {
        let penalized_robots = game_controller_message.

    }

    if player_config.player_number == 
}
