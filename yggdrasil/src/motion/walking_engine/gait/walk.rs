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
        smoothing::parabolic_return,
        step_context::StepContext,
        Side, TargetFootPositions,
    },
    nao::CycleTime,
    sensor::{low_pass_filter::ExponentialLpf, orientation::RobotOrientation},
};

use super::WalkState;

pub(super) struct WalkPlugin;

impl Plugin for WalkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            generate_walk_gait
                .in_set(WalkingEngineSet::GenerateGait)
                .run_if(in_state(Gait::Walking)),
        );
        app.add_systems(OnEnter(Gait::Walking), init_walking_step);

        app.add_systems(
            Update,
            foot_leveling
                .after(WalkingEngineSet::Balance)
                .before(WalkingEngineSet::Finalize)
                .run_if(in_state(Gait::Walking)),
        );
    }
}

fn init_walking_step(
    mut commands: Commands,
    mut step_context: ResMut<StepContext>,
    kinematics: Res<Kinematics>,
    config: Res<WalkingEngineConfig>,
) {
    let start = FootPositions::from_kinematics(
        step_context.planned_step.swing_side,
        &kinematics,
        config.torso_offset,
    );
    step_context.plan_next_step(start, &config);

    commands.insert_resource(WalkState {
        phase: Duration::ZERO,
        planned_step: step_context.planned_step,
    });
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

    let level_orientation = orientation.quaternion() * robot_to_walk_rotation.inverse();
    let (level_roll, level_pitch, _) = level_orientation.euler_angles();

    let weight = logistic_correction_weight(
        state.linear(),
        config.balancing.foot_leveling_phase_shift,
        config.balancing.foot_leveling_decay,
    );

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
