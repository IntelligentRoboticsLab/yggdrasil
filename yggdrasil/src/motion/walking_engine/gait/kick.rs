use std::time::Duration;

use bevy::prelude::*;
use nalgebra::{Translation3, Vector2};

use crate::{
    kinematics::{
        spaces::{LeftSole, RightSole, Robot},
        Kinematics,
    },
    motion::walking_engine::{
        balancing::BalanceAdjustment,
        config::WalkingEngineConfig,
        feet::FootPositions,
        foot_support::FootSupportState,
        schedule::{Gait, WalkingEngineSet},
        smoothing::{parabolic_return, parabolic_step},
        step::PlannedStep,
        step_context::StepContext,
        FootSwitchedEvent, Side, TargetFootPositions,
    },
    nao::CycleTime,
    prelude::Sensor,
    sensor::{low_pass_filter::ExponentialLpf, orientation::RobotOrientation},
};

pub(super) struct KickPlugin;

impl Plugin for KickPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<KickState>();
        app.add_systems(
            Sensor,
            update_support_foot
                .after(crate::sensor::fsr::force_sensitive_resistor_sensor)
                .after(WalkingEngineSet::Prepare),
        );
        app.add_systems(
            Update,
            generate_kick_gait
                .in_set(WalkingEngineSet::GenerateGait)
                .run_if(in_state(Gait::Kicking)),
        );

        app.add_systems(
            Update,
            foot_leveling
                .after(WalkingEngineSet::Balance)
                .before(WalkingEngineSet::Finalize)
                .run_if(in_state(Gait::Kicking)),
        );
    }
}

#[derive(Debug, Clone, Resource)]
struct KickState {
    phase: Duration,
    planned_step: PlannedStep,
    hip_pitch_override: f32,
    ankle_pitch_override: f32,
}

impl Default for KickState {
    fn default() -> Self {
        Self {
            phase: Duration::ZERO,
            planned_step: PlannedStep::default(),
            hip_pitch_override: 0.0,
            ankle_pitch_override: 0.0,
        }
    }
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

    /// Calculate hip and ankle pitch overrides based on the current phase
    fn calculate_joint_overrides(&mut self) {
        let linear = self.linear();

        // Only apply overrides during the middle phase of the kick (20% to 80%)
        if linear < 0.2 {
            // Ramp up from zero
            let factor = linear / 0.2;
            self.hip_pitch_override = -0.4 * factor;
            self.ankle_pitch_override = 0.1 * factor;
        } else if linear < 0.6 {
            // Full strength in middle phase
            self.hip_pitch_override = -0.4;
            self.ankle_pitch_override = 0.1;
        } else if linear < 0.8 {
            // Ramp down to zero
            let factor = (0.8 - linear) / 0.2;
            self.hip_pitch_override = -0.4 * factor;
            self.ankle_pitch_override = 0.1 * factor;
        } else {
            // Zero at the end
            self.hip_pitch_override = 0.0;
            self.ankle_pitch_override = 0.0;
        }
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

        event.send(FootSwitchedEvent {
            new_support: foot_support.support_side(),
            new_swing: foot_support.swing_side(),
        });
    }
}

fn generate_kick_gait(
    mut state: ResMut<KickState>,
    mut target_positions: ResMut<TargetFootPositions>,
    cycle_time: Res<CycleTime>,
    step_context: Res<StepContext>,
    foot_support: Res<FootSupportState>,
    mut balance_adjustment: ResMut<BalanceAdjustment>,
) {
    state.phase += cycle_time.duration;

    let linear = state.linear();
    let parabolic = state.parabolic();

    // Calculate joint overrides for the kick
    state.calculate_joint_overrides();

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

    // Apply a more aggressive foot lift trajectory for the kick
    let kick_phase_factor = 1.5; // Increase the vertical height for kicks
    let swing_lift = parabolic_return(linear) * planned.swing_foot_height * kick_phase_factor;

    // Apply a forward push to the kick motion, proportional to the forward step distance
    let forward_push_factor = match foot_support.swing_side() {
        Side::Left => planned.step.forward.max(0.0) * 1.5, // Increase forward movement
        Side::Right => planned.step.forward.max(0.0) * 1.5,
    };

    let kick_forward_push = if linear > 0.3 && linear < 0.7 {
        // Apply the forward push in the middle of the kick
        let kick_phase = (linear - 0.3) / 0.4; // Normalized phase [0,1] during kick section
        let push_strength = parabolic_return(kick_phase) * forward_push_factor;
        push_strength
    } else {
        0.0
    };

    // Apply joint overrides to the swinging leg
    match foot_support.swing_side() {
        Side::Left => {
            left.translation.z = swing_lift;
            if linear > 0.2 && linear < 0.8 {
                left.translation.x += kick_forward_push;
                balance_adjustment.apply_swing_leg_adjustments(
                    Side::Left,
                    state.hip_pitch_override,
                    state.ankle_pitch_override,
                );
            }
            right.translation.z = 0.0;
        }
        Side::Right => {
            right.translation.z = swing_lift;
            if linear > 0.2 && linear < 0.8 {
                right.translation.x += kick_forward_push;
                balance_adjustment.apply_swing_leg_adjustments(
                    Side::Right,
                    state.hip_pitch_override,
                    state.ankle_pitch_override,
                );
            }
            left.translation.z = 0.0;
        }
    }

    **target_positions = FootPositions {
        left: left.into(),
        right: right.into(),
    };
}

#[derive(Debug, Clone)]
struct FootLevelingState {
    state: ExponentialLpf<2>,
}

impl Default for FootLevelingState {
    fn default() -> Self {
        Self {
            state: ExponentialLpf::new(0.8),
        }
    }
}

fn foot_leveling(
    state: Res<KickState>,
    foot_support: Res<FootSupportState>,
    orientation: Res<RobotOrientation>,
    kinematics: Res<Kinematics>,
    config: Res<WalkingEngineConfig>,
    mut balance_adjustment: ResMut<BalanceAdjustment>,
    mut foot_leveling: Local<FootLevelingState>,
) {
    let hip_height = match foot_support.support_side() {
        Side::Left => kinematics.left_hip_height(),
        Side::Right => kinematics.right_hip_height(),
    };

    let offset = Translation3::new(config.torso_offset, 0., hip_height);
    let left_foot = kinematics.isometry::<LeftSole, Robot>().inner * offset;
    let right_foot = kinematics.isometry::<RightSole, Robot>().inner * offset;

    let robot_to_kick_rotation = match foot_support.support_side() {
        Side::Left => left_foot.rotation,
        Side::Right => right_foot.rotation,
    };

    let level_orientation = orientation.quaternion() * robot_to_kick_rotation.inverse();
    let (level_roll, level_pitch, _) = level_orientation.euler_angles();

    // Use a different weight curve for kicking to ensure stability
    // We want stronger correction in the middle of the kick when the swing foot is at its apex
    let phase = state.linear();
    let weight = if phase < 0.3 || phase > 0.7 {
        // Regular weight at beginning and end
        logistic_correction_weight(
            phase,
            config.balancing.foot_leveling_phase_shift,
            config.balancing.foot_leveling_decay,
        )
    } else {
        // Enhanced stability in middle of kick
        0.9 * logistic_correction_weight(
            phase,
            config.balancing.foot_leveling_phase_shift,
            config.balancing.foot_leveling_decay,
        )
    };

    let target_roll = -level_roll * weight;
    let target_pitch = -level_pitch * weight;

    let target_values = foot_leveling
        .state
        .update(Vector2::new(target_roll, target_pitch));

    balance_adjustment.apply_foot_leveling(
        foot_support.swing_side(),
        target_values.x,
        target_values.y,
    );
}

/// Weighing function for the foot leveling.
///
/// This is a logistic decay function (sigmoid), and returns a value between 0-1,
/// which is used to weigh the impact of foot leveling.
///
/// View the function in desmos [here](https://www.desmos.com/calculator/akfitz58we).
fn logistic_correction_weight(phase: f32, phase_shift: f32, decay: f32) -> f32 {
    let decayed_phase = (-decay * (phase - phase_shift)).exp();
    let factor = 1.0 / (1.0 + decayed_phase);

    (1.0 - factor).clamp(0.0, 1.0)
}
