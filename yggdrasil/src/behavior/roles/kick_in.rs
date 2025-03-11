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
const DISTANCE_TO_SETPOSITION: f32 = 0.4;

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
    let set_robot_position = layout_config
        .set_positions
        .player(player_config.player_number);

    let set_position = set_robot_position.isometry.translation.vector;

    // Check distance to the set position
    let set_point = Point2::new(set_position.x, set_position.y);
    let close_to_target = pose.distance_to(&set_point) < DISTANCE_TO_SETPOSITION;

    // Check if the robot it is facing it's own side of the field. If in front of him the x is going negative
    let aligned_with_rotation = (pose.world_rotation() - set_robot_position.isometry.rotation.angle()).abs() < 0.2;

    // Check if the robot sees a ball and is close to it
    let ball_distance = pose.distance_to(&pose);

    if let Some(game_state) = game_controller_message.as_deref() {
        let penalized_robots = game_state.is_penalized(4, player_config.team_number);

    }


}
