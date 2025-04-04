use bevy::prelude::*;
use nidhogg::types::{FillExt, HeadJoints};

use nalgebra::Point3;

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState},
    core::config::{layout::LayoutConfig, showtime::PlayerConfig},
    localization::RobotPose,
    motion::{
        step_planner::{StepPlanner, Target},
        walking_engine::{step_context::StepContext, StandingHeight},
    },
    nao::{NaoManager, Priority},
};

pub struct WalkToSetBehaviorPlugin;

impl Plugin for WalkToSetBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, walk_to_set.run_if(in_behavior::<WalkToSet>));
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
    layout_config: Res<LayoutConfig>,
    player_config: Res<PlayerConfig>,
    mut step_planner: ResMut<StepPlanner>,
    mut step_context: ResMut<StepContext>,
    mut nao_manager: ResMut<NaoManager>,
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
        .is_none_or(|current_target| *current_target == target)
    {
        step_planner.set_absolute_target(target);
    }

    if let Some(step) = step_planner.plan(&pose) {
        step_context.request_walk(step);
    } else {
        let look_at = pose.get_look_at_absolute(&Point3::origin());
        nao_manager.set_head(
            look_at,
            HeadJoints::fill(NaoManager::HEAD_STIFFNESS),
            Priority::default(),
        );

        step_context.request_stand_with_height(StandingHeight::MAX);
    }
}
