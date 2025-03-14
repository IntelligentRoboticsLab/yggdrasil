use bevy::prelude::*;
use nidhogg::types::{FillExt, LegJoints};

use crate::motion::walking_engine::{
    balancing::BalanceAdjustment,
    config::WalkingEngineConfig,
    feet::FootPositions,
    hips::HipHeight,
    schedule::{Gait, WalkingEngineSet},
    TargetFootPositions, TargetLegStiffness,
};

pub(super) struct StandGaitPlugin;

impl Plugin for StandGaitPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            generate_stand_gait
                .in_set(WalkingEngineSet::GenerateGait)
                .run_if(in_state(Gait::Standing)),
        );
    }
}

fn generate_stand_gait(
    mut target: ResMut<TargetFootPositions>,
    mut hip_height: ResMut<HipHeight>,
    mut target_stiffness: ResMut<TargetLegStiffness>,
    config: Res<WalkingEngineConfig>,
    mut balancing: ResMut<BalanceAdjustment>,
) {
    // always set the foot offsets to 0,0,0.
    **target = FootPositions::default();

    hip_height.request(config.hip_height.walking_hip_height);
    **target_stiffness = LegJoints::fill(config.walking_leg_stiffness);

    let _ = balancing.prepare();
}
