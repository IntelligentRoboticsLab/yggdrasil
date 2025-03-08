use std::time::Duration;

use bevy::prelude::*;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_with::{DurationMilliSeconds, serde_as};

use crate::{
    motion::walking_engine::{
        FootSwitchedEvent, Side, TargetFootPositions,
        balancing::BalanceAdjustment,
        config::{KickingConfig, WalkingEngineConfig},
        feet::FootPositions,
        foot_support::FootSupportState,
        schedule::{Gait, WalkingEngineSet},
        smoothing::{parabolic_return, parabolic_step},
        step::PlannedStep,
        step_context::{KickVariant, StepContext},
    },
    nao::CycleTime,
    prelude::Sensor,
};

pub(super) struct KickPlugin;

impl Plugin for KickPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KickState>();
        app.add_systems(
            Sensor,
            update_support_foot
                .after(crate::sensor::fsr::update_force_sensitive_resistor_sensor)
                .after(WalkingEngineSet::Prepare),
        );
        app.add_systems(
            Update,
            generate_kick_gait
                .in_set(WalkingEngineSet::GenerateGait)
                .run_if(in_state(Gait::Kicking)),
        );
    }
}

#[serde_as]
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct JointOverride {
    pub offset: f32,
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub timepoint: Duration,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LegJointsOverrideSequence {
    pub hip_pitch: Vec<JointOverride>,
    pub ankle_pitch: Vec<JointOverride>,
}

#[derive(Debug, Clone, Default)]
pub struct LegJointsOverride {
    pub hip_pitch_override: f32,
    pub ankle_pitch_override: f32,
}

#[derive(Debug, Clone, Resource, Default)]
struct KickState {
    phase: Duration,
    planned_step: PlannedStep,
}

impl KickState {
    /// Get a value from [0, 1] describing the linear progress of the current step.
    ///
    /// This value is based on the current `phase` and `planned_duration`, and will always be
    /// within the inclusive range from 0 to 1.   
    #[inline]
    #[must_use]
    fn linear(&self) -> f32 {
        (self.phase.as_secs_f32() / self.planned_step.duration.as_secs_f32()).clamp(0.0, 1.0)
    }

    /// Get a value from [0, 1] describing the position of the current step, along a parabolic path.
    ///
    /// See [`parabolic_step`] for more.
    #[inline]
    #[must_use]
    fn parabolic(&self) -> f32 {
        parabolic_step(self.linear())
    }

    fn compute_leg_joint_override(
        &self,
        leg_joints_override: &LegJointsOverrideSequence,
        phase: Duration,
    ) -> LegJointsOverride {
        let hip_pitch_override = self.compute_override(&leg_joints_override.hip_pitch, phase);
        let ankle_pitch_override = self.compute_override(&leg_joints_override.ankle_pitch, phase);
        LegJointsOverride {
            hip_pitch_override,
            ankle_pitch_override,
        }
    }

    // HULKs compute overide, using lerp to compute joint override in between phases
    fn compute_override(&self, overrides: &[JointOverride], t: Duration) -> f32 {
        let Some((start, end)) = overrides
            .iter()
            .tuple_windows()
            .find(|(start, end)| (start.timepoint..end.timepoint).contains(&t))
        else {
            return 0.0;
        };

        let linear_time = self.linear();
        f32::lerp(linear_time, start.offset, end.offset)
    }
}

/// System that checks whether the swing foot should be updated, and does so when possible.
fn update_support_foot(
    mut state: ResMut<KickState>,
    mut foot_support: ResMut<FootSupportState>,
    mut event: EventWriter<FootSwitchedEvent>,
    config: Res<WalkingEngineConfig>,
) {
    // only switch if we've completed the minimum ratio of the step
    let is_switch_allowed = state.linear() > config.minimum_step_duration_ratio;

    let foot_switched = is_switch_allowed && foot_support.predicted_or_switched();

    if foot_switched {
        state.phase = Duration::ZERO;
        foot_support.switch_support_side();

        event.write(FootSwitchedEvent {
            new_support: foot_support.support_side(),
            new_swing: foot_support.swing_side(),
        });
    }
}

fn generate_kick_step_overrides(
    step_index: usize,
    kick_variant: KickVariant,
    kicking_config: &KickingConfig,
) -> Option<(&LegJointsOverrideSequence, &LegJointsOverrideSequence)> {
    let kick_settings = match kick_variant {
        KickVariant::Forward => &kicking_config.forward,
        KickVariant::Turn => &kicking_config.turn,
        KickVariant::Side => &kicking_config.side,
    };
    let step = kick_settings.kick_steps.get(step_index)?;

    /*
    let support_overrides = [
        LegJointsOverrideSequence {
            hip_pitch: vec![],
            ankle_pitch: vec![],
        },
        LegJointsOverrideSequence {
            hip_pitch: vec![
                JointOverride {
                    offset: 0.0,
                    timepoint: Duration::from_millis(0),
                },
                JointOverride {
                    offset: -0.02,
                    timepoint: Duration::from_millis(100),
                },
                JointOverride {
                    offset: -0.04,
                    timepoint: Duration::from_millis(150),
                },
                JointOverride {
                    offset: -0.02,
                    timepoint: Duration::from_millis(240),
                },
            ],
            ankle_pitch: vec![
                JointOverride {
                    offset: 0.0,
                    timepoint: Duration::from_millis(0),
                },
                JointOverride {
                    offset: 0.0,
                    timepoint: Duration::from_millis(50),
                },
                JointOverride {
                    offset: 0.0,
                    timepoint: Duration::from_millis(150),
                },
                JointOverride {
                    offset: 0.0,
                    timepoint: Duration::from_millis(200),
                },
            ],
        },
        LegJointsOverrideSequence {
            hip_pitch: vec![],
            ankle_pitch: vec![],
        },
        LegJointsOverrideSequence {
            hip_pitch: vec![],
            ankle_pitch: vec![],
        },
    ];

    let swing_overrides = [
        LegJointsOverrideSequence {
            hip_pitch: vec![],
            ankle_pitch: vec![],
        },
        LegJointsOverrideSequence {
            hip_pitch: vec![
                JointOverride {
                    offset: 0.0,
                    timepoint: Duration::from_millis(0),
                },
                JointOverride {
                    offset: 0.0,
                    timepoint: Duration::from_millis(100),
                },
                JointOverride {
                    offset: -0.4,
                    timepoint: Duration::from_millis(150),
                },
                JointOverride {
                    offset: 0.0,
                    timepoint: Duration::from_millis(240),
                },
            ],
            ankle_pitch: vec![
                JointOverride {
                    offset: 0.0,
                    timepoint: Duration::from_millis(0),
                },
                JointOverride {
                    offset: 0.0,
                    timepoint: Duration::from_millis(50),
                },
                JointOverride {
                    offset: 0.1,
                    timepoint: Duration::from_millis(150),
                },
                JointOverride {
                    offset: 0.0,
                    timepoint: Duration::from_millis(200),
                },
            ],
        },
        LegJointsOverrideSequence {
            hip_pitch: vec![],
            ankle_pitch: vec![],
        },
        LegJointsOverrideSequence {
            hip_pitch: vec![],
            ankle_pitch: vec![],
        },
    ];

    let support_override = match support_overrides.get(step_index) {
        Some(support_override) => support_override.clone(),
        None => LegJointsOverrideSequence::default(),
    };

    let swing_override = match swing_overrides.get(step_index) {
        Some(swing_override) => swing_override.clone(),
        None => LegJointsOverrideSequence::default(),
    };
    */

    let joint_override = step.joint_override.as_ref()?;
    Some((&joint_override.support, &joint_override.swing))
}

fn generate_kick_gait(
    mut state: ResMut<KickState>,
    mut target_positions: ResMut<TargetFootPositions>,
    cycle_time: Res<CycleTime>,
    step_context: Res<StepContext>,
    foot_support: Res<FootSupportState>,
    mut balance_adjustment: ResMut<BalanceAdjustment>,
    kicking_config: Res<KickingConfig>,
) {
    // Update the time that is passed during the current step
    state.phase += cycle_time.duration;

    let linear = state.linear();
    let parabolic = state.parabolic();

    let (left_t, right_t) = match &foot_support.swing_side() {
        Side::Left => (parabolic, linear),
        Side::Right => (linear, parabolic),
    };

    let planned = step_context.planned_step;
    state.planned_step = planned;
    let start = planned.start;
    let target = planned.target;
    let mut left = start.left.lerp_slerp(&target.left.inner, left_t);
    let mut right = start.right.lerp_slerp(&target.right.inner, right_t);

    let swing_lift = parabolic_return(linear) * planned.swing_foot_height;
    let (left_lift, right_lift) = match &foot_support.swing_side() {
        Side::Left => (swing_lift, 0.),
        Side::Right => (0., swing_lift),
    };

    left.translation.z = left_lift;
    right.translation.z = right_lift;

    **target_positions = FootPositions {
        left: left.into(),
        right: right.into(),
    };

    // TODO(Rick): Temporary place :(
    if let Some(kick_seq) = step_context.active_kick.as_ref() {
        let step_index = kick_seq.step_index;

        if let Some((kick_step_override_support, kick_step_override_swing)) =
            generate_kick_step_overrides(step_index, kick_seq.variant, &kicking_config)
        {
            // Compute the offset for the support and swing leg (ankle and hip pitch)
            let swing_leg_override =
                state.compute_leg_joint_override(kick_step_override_swing, state.phase);
            let support_leg_override =
                state.compute_leg_joint_override(kick_step_override_support, state.phase);

            // Apply the leg joint offsets/overrides using the balance adjustment
            balance_adjustment
                .apply_leg_adjustments(state.planned_step.swing_side, support_leg_override);
            balance_adjustment.apply_leg_adjustments(
                state.planned_step.swing_side.opposite(),
                swing_leg_override,
            );
        }
    }
}
