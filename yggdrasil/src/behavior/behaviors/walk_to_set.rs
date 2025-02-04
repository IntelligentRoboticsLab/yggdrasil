use std::time::Duration;

use bevy::prelude::*;
use nidhogg::types::{FillExt, HeadJoints};

use nalgebra::{Point2, Point3};

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState, Role},
    core::config::{layout::LayoutConfig, showtime::PlayerConfig},
    localization::RobotPose,
    motion::{
        step_planner::{StepPlanner, Target},
        walk::engine::WalkingEngine,
    },
    nao::{NaoManager, Priority},
};

/// To prevent the Goalkeeper from walking into the goalpost, we use this position for a better approach.
const GOAL_KEEPER_PRE_SET_POS: Target = Target {
    position: Point2::new(-2.85, 0.0),
    rotation: None,
};

const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);

pub struct WalkToSetBehaviorPlugin;

impl Plugin for WalkToSetBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, walk_to_set.run_if(in_behavior::<WalkToSet>));
    }
}

/// Walk to the set position of the robot.
/// Only the Goalkeeper will first walk to the pre-set position before walking to the set position.
#[derive(Resource)]
pub struct WalkToSet {
    // pub is_goalkeeper: bool,
}

impl Behavior for WalkToSet {
    const STATE: BehaviorState = BehaviorState::WalkToSet;
}

pub fn walk_to_set(
    pose: Res<RobotPose>,
    layout_config: Res<LayoutConfig>,
    player_config: Res<PlayerConfig>,
    mut step_planner: ResMut<StepPlanner>,
    mut walking_engine: ResMut<WalkingEngine>,
    mut nao_manager: ResMut<NaoManager>,
    role: Res<State<Role>>,
) {
    let set_robot_position = layout_config
        .set_positions
        .player(player_config.player_number);

    let set_position: Point2<f32> = set_robot_position.isometry.translation.vector.into();

    let look_at = pose.get_look_at_absolute(&Point3::new(
        set_position.x,
        set_position.y,
        RobotPose::CAMERA_HEIGHT,
    ));

    nao_manager.set_head_target(
        look_at,
        HEAD_ROTATION_TIME,
        Priority::default(),
        NaoManager::HEAD_STIFFNESS,
    );

    let target = Target {
        position: set_robot_position.isometry.translation.vector.into(),
        rotation: Some(set_robot_position.isometry.rotation),
    };

    let reached_pre_set = !step_planner.has_target()
        || (step_planner
            .current_absolute_target()
            .is_some_and(|current_target| current_target == &GOAL_KEEPER_PRE_SET_POS)
            && !step_planner.reached_target());

    let is_goalkeeper = role.get() == &Role::Goalkeeper;

    if is_goalkeeper && reached_pre_set {
        step_planner.set_absolute_target(GOAL_KEEPER_PRE_SET_POS);
    } else {
        step_planner.set_absolute_target(target);
    }

    if let Some(step) = step_planner.plan(&pose) {
        walking_engine.request_walk(step);
    } else {
        let look_at = pose.get_look_at_absolute(&Point3::origin());
        nao_manager.set_head(look_at, HeadJoints::fill(0.5), Priority::default());

        walking_engine.request_stand();
    }
}
