use bevy::prelude::*;
use nidhogg::types::{FillExt, LegJoints};

use crate::motion::walkv4::{
    config::WalkingEngineConfig,
    feet::FootPositions,
    hips::HipHeight,
    schedule::{Gait, GaitGeneration, MotionSet},
    TargetFootPositions, TargetLegStiffness,
};

pub(super) struct StandGaitPlugin;

impl Plugin for StandGaitPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(Gait::Standing),
            request_sit.in_set(MotionSet::GaitGeneration),
        );
        app.add_systems(
            GaitGeneration,
            generate_stand_gait
                .in_set(MotionSet::GaitGeneration)
                .run_if(in_state(Gait::Standing)),
        );
    }
}

fn request_sit(
    config: Res<WalkingEngineConfig>,
    mut hip_height: ResMut<HipHeight>,
    mut target_stiffness: ResMut<TargetLegStiffness>,
) {
    hip_height.request(config.hip_height);
    **target_stiffness = LegJoints::fill(config.leg_stiffness);
}

fn generate_stand_gait(mut target: ResMut<TargetFootPositions>) {
    // always set the foot offsets to 0,0,0.
    **target = FootPositions::default();
}
