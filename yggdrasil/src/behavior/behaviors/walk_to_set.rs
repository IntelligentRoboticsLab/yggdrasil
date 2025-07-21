use std::time::Instant;

use bevy::prelude::*;
use bifrost::communication::GameControllerMessage;
use nalgebra::UnitComplex;
use nidhogg::types::{FillExt, HeadJoints};

use crate::{
    behavior::{
        BehaviorConfig,
        engine::{Behavior, BehaviorState, in_behavior},
    },
    core::config::{layout::LayoutConfig, showtime::PlayerConfig},
    localization::RobotPose,
    motion::{
        step_planner::{StepPlanner, Target},
        walking_engine::step_context::StepContext,
    },
    nao::HeadMotionManager,
};

#[derive(Resource, Deref)]
struct ObserveStartingTime(Instant);

fn reset_observe_starting_time(mut observe_starting_time: ResMut<ObserveStartingTime>) {
    observe_starting_time.0 = Instant::now();
}

pub struct WalkToSetBehaviorPlugin;

impl Plugin for WalkToSetBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, walk_to_set.run_if(in_behavior::<WalkToSet>));
        app.add_systems(
            OnEnter(BehaviorState::WalkToSet),
            reset_observe_starting_time,
        );
        app.insert_resource(ObserveStartingTime(Instant::now()));
    }
}

/// Walk to the set position of the robot.
/// Only the Goalkeeper will first walk to the pre-set position before walking to the set position.
#[derive(Resource)]
pub struct WalkToSet;

impl Behavior for WalkToSet {
    const STATE: BehaviorState = BehaviorState::WalkToSet;
}

const INDIRECT_KICK_POSITION: [f32; 2] = [0.4, 0.4];
const INDIRECT_KICK_ROTATION: f32 = -135.0;

#[allow(clippy::too_many_arguments)]
fn walk_to_set(
    pose: Res<RobotPose>,
    (layout_config, player_config): (Res<LayoutConfig>, Res<PlayerConfig>),
    mut step_planner: ResMut<StepPlanner>,
    mut step_context: ResMut<StepContext>,
    mut head_motion_manager: ResMut<HeadMotionManager>,
    config: Res<BehaviorConfig>,
    gamecontrollermessage: Res<GameControllerMessage>,
) {
    let set_robot_position = layout_config
        .set_positions
        .player(player_config.player_number);

    let mut target = Target {
        position: set_robot_position.isometry.translation.vector.into(),
        rotation: Some(set_robot_position.isometry.rotation),
    };

    if player_config.player_number == 5
        && gamecontrollermessage.kicking_team == player_config.team_number
    {
        target = Target {
            position: INDIRECT_KICK_POSITION.into(),
            rotation: Some(UnitComplex::new(INDIRECT_KICK_ROTATION.to_radians())),
        };
    }

    if step_planner
        .current_absolute_target()
        .is_none_or(|current_target| *current_target != target)
    {
        step_planner.set_absolute_target(target);
    }

    if let Some(step) = step_planner.plan(&pose) {
        step_context.request_walk(step);
    } else {
        step_context.request_stand();
    }

    head_motion_manager.request_look_around();
}
