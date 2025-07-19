use std::time::Instant;

use bevy::prelude::*;

use crate::{
    behavior::engine::{Behavior, BehaviorState, in_behavior},
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

fn walk_to_set(
    pose: Res<RobotPose>,
    (layout_config, player_config): (Res<LayoutConfig>, Res<PlayerConfig>),
    mut step_planner: ResMut<StepPlanner>,
    mut step_context: ResMut<StepContext>,
    mut head_motion_manager: ResMut<HeadMotionManager>,
) {
    let set_robot_position = layout_config
        .set_positions
        .player(player_config.player_number);

    let target = Target {
        position: set_robot_position.isometry.translation.vector.into(),
        rotation: Some(set_robot_position.isometry.rotation),
    };

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
