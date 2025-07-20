use bevy::prelude::*;

use crate::{
    behavior::engine::{Behavior, BehaviorState, in_behavior},
    motion::walking_engine::{StandingHeight, step_context::StepContext},
    nao::HeadMotionManager,
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

fn stand(
    mut step_context: ResMut<StepContext>,
    mut head_motion_manager: ResMut<HeadMotionManager>,
) {
    step_context.request_stand_with_height(StandingHeight::MAX);
    head_motion_manager.request_neutral();
}
