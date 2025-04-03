use bevy::prelude::*;
use nidhogg::{
    types::{FillExt, JointArray, LegJoints},
    NaoControlMessage, NaoState,
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
    prelude::*,
};

pub(super) struct StandGaitPlugin;

impl Plugin for StandGaitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JointCurrentOptimizer>();
        app.add_systems(
            Update,
            generate_stand_gait
                .in_set(WalkingEngineSet::GenerateGait)
                .run_if(in_state(Gait::Standing)),
        );

        app.add_systems(
            PreWrite,
            energy_efficient_stand
                .after(crate::nao::finalize)
                .run_if(in_state(Gait::Standing)),
        );

        app.add_systems(OnExit(Gait::Standing), reset_joint_current_optimizer);
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

/// Joint position encoders have a resolution of 1/4096, but we apply a smaller
/// incremental adjustment of 0.01/4096, scaled by cycle time.
///
/// During longer cycles, smaller adjustments are enough, as the motor controllers run as part of HAL, not yggdrasil.
const REDUCTION: f32 = 0.01 / 4096.;

#[derive(Debug, Resource, Default)]
struct JointCurrentOptimizer {
    has_reach_minimum_current: bool,
    joint_offsets: JointArray<f32>,
}

fn reset_joint_current_optimizer(mut state: ResMut<JointCurrentOptimizer>) {
    *state = JointCurrentOptimizer::default();
}

/// System that optimises the requested joint positions based on the maximum current.
///
/// Based on the implementation as described in the Berlin United 2019 Tech Report.
fn energy_efficient_stand(
    nao_state: Res<NaoState>,
    mut control_msg: ResMut<NaoControlMessage>,
    mut state: ResMut<JointCurrentOptimizer>,
    cycle_time: Res<CycleTime>,
) {
    let (joint_idx, max_current) = nao_state
        .current
        .into_iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .unwrap();

    state.has_reach_minimum_current = if *max_current < MIN_CURRENT {
        true
    } else if *max_current > MAX_CURRENT {
        false
    } else {
        state.has_reach_minimum_current
    };

    if !state.has_reach_minimum_current {
        let max_adjustment = REDUCTION / cycle_time.duration.as_secs_f32();

        if let Some(joint_offset) = state.joint_offsets.get_mut(joint_idx) {
            let requested_joints = control_msg.position.as_array();
            let measured_joints = nao_state.position.as_array();

            *joint_offset += (requested_joints[joint_idx] - measured_joints[joint_idx])
                .clamp(-max_adjustment, max_adjustment);
        }
    }

    control_msg.position = control_msg
        .position
        .clone()
        .zip(state.joint_offsets.clone())
        .map(|(position, offset)| position + offset);
}
