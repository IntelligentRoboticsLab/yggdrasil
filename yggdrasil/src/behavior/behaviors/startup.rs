use bevy::prelude::*;

use crate::{
    behavior::engine::{Behavior, BehaviorState, in_behavior},
    kinematics::Kinematics,
    motion::walking_engine::{config::WalkingEngineConfig, step_context::StepContext},
};

pub struct StartUpBehaviorPlugin;

impl Plugin for StartUpBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, startup.run_if(in_behavior::<StartUp>));
    }
}

/// This is the startup behavior of the robot.
/// In this state the robot does nothing and retains its previous position.
#[derive(Resource)]
pub struct StartUp;

impl Behavior for StartUp {
    const STATE: BehaviorState = BehaviorState::StartUp;
}

fn startup(
    mut step_context: ResMut<StepContext>,
    config: Res<WalkingEngineConfig>,
    kinematics: Res<Kinematics>,
) {
    let hip_height = kinematics.left_hip_height();

    if hip_height >= config.hip_height.max_sitting_hip_height {
        step_context.request_stand();
    } else {
        step_context.request_sit();
    }
}
