use bevy::prelude::*;
use nalgebra::Point2;
use nidhogg::types::HeadJoints;
use std::{ops::Index, time::Duration};

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState},
    core::config::{layout::LayoutConfig, showtime::PlayerConfig},
    motion::walking_engine::step_context::StepContext,
    nao::{NaoManager, Priority},
};

const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);

#[derive(Resource)]
pub struct InitialStandLook;

impl Behavior for InitialStandLook {
    const STATE: BehaviorState = BehaviorState::InitialStandLook;
}

pub struct InitialStandLookBehaviorPlugin;
impl Plugin for InitialStandLookBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            initial_stand_look.run_if(in_behavior::<InitialStandLook>),
        );
    }
}

pub fn initial_stand_look(
    mut nao_manager: ResMut<NaoManager>,
    mut step_context: ResMut<StepContext>,
    layout_config: Res<LayoutConfig>,
    player_config: Res<PlayerConfig>,
) {
    let initial_position = layout_config
        .as_ref()
        .initial_positions
        .index(player_config.player_number as usize);

    let look_target = initial_position
        .isometry
        .inverse_transform_point(&Point2::origin());

    let yaw = look_target.y.atan2(look_target.x);

    let initial_stand_look = HeadJoints { yaw, pitch: 0.0 };

    nao_manager.set_head_target(
        initial_stand_look,
        HEAD_ROTATION_TIME,
        Priority::default(),
        NaoManager::HEAD_STIFFNESS,
    );
    step_context.request_stand();
}
