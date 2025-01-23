use bevy::prelude::*;

use crate::motion::{
    walk::WalkingEngineConfig,
    walkv4::{
        feet::FootPositions,
        hips::HipHeight,
        scheduling::{MotionSet, MotionState},
        TargetFootPositions,
    },
};

pub(super) struct SitGaitPlugin;

impl Plugin for SitGaitPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(MotionState::Sitting),
            request_sit.in_set(MotionSet::GaitGeneration),
        );
        app.add_systems(
            Update,
            generate_foot_positions
                .in_set(MotionSet::GaitGeneration)
                .run_if(in_state(MotionState::Sitting)),
        );
    }
}

fn request_sit(config: Res<WalkingEngineConfig>, mut hip_height: ResMut<HipHeight>) {
    hip_height.request(config.sitting_hip_height);
}

fn generate_foot_positions(mut target: ResMut<TargetFootPositions>) {
    // always set the foot offsets to 0,0,0.
    **target = FootPositions::default();
}
