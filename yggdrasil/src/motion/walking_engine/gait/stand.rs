use bevy::prelude::*;
use nidhogg::{
    types::{FillExt, JointArray, LegJoints},
    NaoState,
};

use crate::{
    motion::walking_engine::{
        config::WalkingEngineConfig,
        feet::FootPositions,
        hips::HipHeight,
        schedule::{Gait, WalkingEngineSet},
        TargetFootPositions, TargetLegStiffness,
    },
    nao::CycleTime,
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
) {
    // always set the foot offsets to 0,0,0.
    **target = FootPositions::default();

    hip_height.request(config.hip_height.walking_hip_height);
    **target_stiffness = LegJoints::fill(config.walking_leg_stiffness);
}

const MIN_CURRENT: f32 = 0.09;
const MAX_CURRENT: f32 = 0.12;
const REDUCTION: f32 = 0.000005;

/// System that optimises the requested joint positions based on the maximum current.
///
/// Based on the implementation as described in the Berlin United 2019 Tech Report.
fn energy_efficient_stand(
    state: Res<NaoState>,
    mut minimum_reached: Local<bool>,
    mut joint_offsets: Local<JointArray<f32>>,
    cycle_time: Res<CycleTime>,
) {
    let (joint_idx, max_current) = state
        .current
        .into_iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .expect("e");

    *minimum_reached = if *max_current < MIN_CURRENT {
        true
    } else if *max_current > MAX_CURRENT {
        false
    } else {
        *minimum_reached
    };

    if !*minimum_reached {
        let max_adjustment = REDUCTION / cycle_time.duration.as_secs_f32();
        // TODO(gijsd): apply to joint at index `joint_idx`
    }
}
