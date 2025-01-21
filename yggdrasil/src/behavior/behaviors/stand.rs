use std::time::Duration;

use bevy::prelude::*;
use nidhogg::types::HeadJoints;

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState},
    motion::walk::engine::WalkingEngine,
    nao::{NaoManager, Priority},
};

pub struct StandBehaviorPlugin;

impl Plugin for StandBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, stand.run_if(in_behavior::<Stand>));
    }
}

#[derive(Resource)]
pub struct Stand;
impl Behavior for Stand {
    const STATE: BehaviorState = BehaviorState::Stand;
}

const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);

pub fn stand(mut walking_engine: ResMut<WalkingEngine>, mut nao_manager: ResMut<NaoManager>) {
    walking_engine.request_stand();
    walking_engine.end_step_phase();

    nao_manager.set_head_target(
        HeadJoints::default(),
        HEAD_ROTATION_TIME,
        Priority::default(),
        NaoManager::HEAD_STIFFNESS,
    );
}
