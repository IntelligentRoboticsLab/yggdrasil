use bevy::prelude::*;
use nidhogg::{NaoControlMessage, NaoState, types::JointArray};

use crate::{nao::CycleTime, prelude::PreWrite};

const MIN_CURRENT: f32 = 0.09;
const MAX_CURRENT: f32 = 0.12;

/// Joint position encoders have a resolution of 1/4096, but we apply a smaller
/// incremental adjustment of 0.01/4096, scaled by cycle time.
///
/// During longer cycles, smaller adjustments are enough, as the motor controllers run as part of HAL, not yggdrasil.
const REDUCTION: f32 = 0.01 / 4096.0;

pub struct EnergyOptimizerPlugin;

impl Plugin for EnergyOptimizerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<JointCurrentOptimizer>();
        // app.add_systems(
        //     PreWrite,
        //     optimize_joint_currents
        //         .after(crate::nao::finalize)
        //         .run_if(should_optimize_joint_current),
        // );
    }
}

#[derive(Debug, Resource, Default)]
pub struct JointCurrentOptimizer {
    enabled: bool,
    has_reached_minimum_current: bool,
    joint_offsets: JointArray<f32>,
}

/// Run condition that returns `true` when the [`JointCurrentOptimizer`] is enabled.
fn should_optimize_joint_current(state: Res<JointCurrentOptimizer>) -> bool {
    state.enabled
}

/// Extension trait for toggling joint energy optimization.
pub trait EnergyOptimizerExt<'w, 's> {
    /// Enables joint current optimization, using [`JointCurrentOptimizer`].
    fn optimize_joint_currents(&mut self);

    /// Resets the [`JointCurrentOptimizer`] state.
    fn reset_joint_current_optimizer(&mut self);
}

impl<'w, 's> EnergyOptimizerExt<'w, 's> for Commands<'w, 's> {
    fn optimize_joint_currents(&mut self) {
        self.queue(|world: &mut World| {
            world.resource_mut::<JointCurrentOptimizer>().enabled = true;
        });
    }

    fn reset_joint_current_optimizer(&mut self) {
        self.queue(|world: &mut World| {
            world.init_resource::<JointCurrentOptimizer>();
        });
    }
}

/// System that resets the joint current optimizer state.
pub fn reset_joint_current_optimizer(mut state: ResMut<JointCurrentOptimizer>) {
    *state = JointCurrentOptimizer::default();
}

/// System that optimises the requested joint positions based on the maximum current.
///
/// Based on the implementation as described in the Berlin United 2019 Tech Report.
fn optimize_joint_currents(
    nao_state: Res<NaoState>,
    mut control_msg: ResMut<NaoControlMessage>,
    mut state: ResMut<JointCurrentOptimizer>,
    cycle_time: Res<CycleTime>,
) {
    let (joint_idx, max_current) = nao_state
        .current
        .as_array_ref()
        .into_iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b))
        .unwrap();

    state.has_reached_minimum_current = if *max_current < MIN_CURRENT {
        true
    } else if *max_current > MAX_CURRENT {
        false
    } else {
        state.has_reached_minimum_current
    };

    if !state.has_reached_minimum_current {
        let max_adjustment = REDUCTION / cycle_time.duration.as_secs_f32();

        if let Some(joint_offset) = state.joint_offsets.get_mut(joint_idx) {
            let requested_joints = control_msg.position.as_array_ref();
            let measured_joints = nao_state.position.as_array_ref();

            *joint_offset += (requested_joints[joint_idx] - measured_joints[joint_idx])
                .clamp(-max_adjustment, max_adjustment);
        }
    }

    control_msg.position = control_msg
        .position
        .clone()
        .zip(state.joint_offsets.clone())
        .map(|(position, offset)| position + offset);

    // reset the enabled state, so it can be enabled again next cycle if needed.
    state.enabled = false;
}
