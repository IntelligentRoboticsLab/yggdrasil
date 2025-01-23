use bevy::prelude::*;

use crate::motion::walkv4::{
    feet::FootPositions,
    scheduling::{MotionSet, MotionState},
    TargetFootPositions,
};

pub(super) struct StandGaitPlugin;

impl Plugin for StandGaitPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            generate_foot_positions
                .in_set(MotionSet::GaitGeneration)
                .run_if(in_state(MotionState::Standing)),
        );
    }
}

fn generate_foot_positions(mut target: ResMut<TargetFootPositions>) {
    // always set the foot offsets to 0,0,0.
    **target = FootPositions::default();
}
