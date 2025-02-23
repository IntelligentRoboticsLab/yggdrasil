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

pub(super) struct WalkPlugin;

impl Plugin for WalkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WalkState>();
        app.add_systems(
            Sensor,
            update_support_foot
                .after(crate::sensor::fsr::force_sensitive_resistor_sensor)
                .after(WalkingEngineSet::Prepare),
        );
        app.add_systems(
            Update,
            generate_walk_gait
                .in_set(WalkingEngineSet::GenerateGait)
                .run_if(in_state(Gait::Walking)),
        );

        app.add_systems(
            Update,
            foot_leveling
                .after(WalkingEngineSet::Balance)
                .before(WalkingEngineSet::Finalize)
                .run_if(in_state(Gait::Walking)),
        );
    }
}

#[derive(Debug, Clone, Resource)]
struct WalkState {
    phase: Duration,
    planned_step: PlannedStep,
}

impl Default for WalkState {
    fn default() -> Self {
        Self {
            phase: Duration::ZERO,
            planned_step: PlannedStep::default(),
        }
    }
}

impl WalkState {
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
}

/// System that checks whether the swing foot should be updated, and does so when possible.
fn update_support_foot(
    mut state: ResMut<WalkState>,
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

fn generate_walk_gait(
    mut state: ResMut<WalkState>,
    mut target_positions: ResMut<TargetFootPositions>,
    cycle_time: Res<CycleTime>,
    step_context: Res<StepContext>,
    foot_support: Res<FootSupportState>,
) {
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
    state: Res<WalkState>,
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

    let robot_to_walk_rotation = match foot_support.support_side() {
        Side::Left => left_foot.rotation,
        Side::Right => right_foot.rotation,
    };

    // Calculate level orientation
    let level_orientation = orientation.quaternion() * robot_to_walk_rotation.inverse();
    let (level_roll, level_pitch, _) = level_orientation.euler_angles();

    // Calculate return factor based on step phase
    let return_factor = ((state.linear() - 0.5).max(0.0) * 2.0).powi(2);

    // Calculate target angles
    let target_roll = -level_roll * (1.0 - return_factor);
    let target_pitch = -level_pitch * (1.0 - return_factor);

    let target_values = foot_leveling
        .state
        .update(Vector2::new(target_roll, target_pitch));

    balance_adjustment.apply_foot_leveling(
        foot_support.swing_side(),
        target_values.x,
        target_values.y,
    );
}
