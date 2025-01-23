use bevy::prelude::*;

use crate::motion::walkv4::{
    config::WalkingEngineConfig,
    feet::FootPositions,
    hips::HipHeight,
    scheduling::{MotionSet, MotionState},
    TargetFootPositions,
};

pub(super) struct StandGaitPlugin;

impl Plugin for StandGaitPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(MotionState::Standing),
            request_sit.in_set(MotionSet::GaitGeneration),
        );
        app.add_systems(
            Update,
            generate_foot_positions
                .in_set(MotionSet::GaitGeneration)
                .run_if(in_state(MotionState::Standing)),
        );
    }
}

fn request_sit(config: Res<WalkingEngineConfig>, mut hip_height: ResMut<HipHeight>) {
    hip_height.request(config.hip_height);
}

fn generate_foot_positions(mut target: ResMut<TargetFootPositions>) {
    // always set the foot offsets to 0,0,0.
    **target = FootPositions::default();
}
