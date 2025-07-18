use std::time::Instant;

use bevy::prelude::*;
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
    nao::{NaoManager, Priority},
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
    mut nao_manager: ResMut<NaoManager>,
    observe_starting_time: Res<ObserveStartingTime>,
    config: Res<BehaviorConfig>,
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

    let observe_config = &config.observe;

    look_around(
        &mut nao_manager,
        **observe_starting_time,
        observe_config.head_rotation_speed,
        observe_config.head_yaw_max,
        observe_config.head_pitch_max,
    );
}

fn look_around(
    nao_manager: &mut NaoManager,
    starting_time: Instant,
    rotation_speed: f32,
    yaw_multiplier: f32,
    pitch_multiplier: f32,
) {
    // Used to parameterize the yaw and pitch angles, multiplying with a large
    // rotation speed will make the rotation go faster.
    let movement_progress = starting_time.elapsed().as_secs_f32() * rotation_speed;
    let yaw = (movement_progress).sin() * yaw_multiplier;
    let pitch = (movement_progress * 2.0 + std::f32::consts::FRAC_PI_2)
        .sin()
        .max(0.0)
        * pitch_multiplier;

    let position = HeadJoints { yaw, pitch };

    nao_manager.set_head(
        position,
        HeadJoints::fill(NaoManager::HEAD_STIFFNESS),
        Priority::default(),
    );
}
