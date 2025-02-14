use std::time::Duration;

use bevy::prelude::*;
use nidhogg::types::HeadJoints;

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState},
    motion::walkv4::step_manager::StepContext,
    nao::{NaoManager, Priority},
};

const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);

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

pub fn stand(mut step_context: ResMut<StepContext>, mut nao_manager: ResMut<NaoManager>) {
    step_context.request_stand();

    nao_manager.set_head_target(
        HeadJoints::default(),
        HEAD_ROTATION_TIME,
        Priority::default(),
        NaoManager::HEAD_STIFFNESS,
    );
}
