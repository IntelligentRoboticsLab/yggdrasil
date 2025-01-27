use std::time::Duration;

use bevy::prelude::*;
use nidhogg::types::ForceSensitiveResistors;

use crate::{
    kinematics::Kinematics,
    motion::{
        walk::smoothing::{parabolic_return, parabolic_step},
        walkv4::{
            config::WalkingEngineConfig,
            feet::FootPositions,
            scheduling::{MotionSet, MotionState},
            step::Step,
            Side, SwingFoot, TargetFootPositions, TORSO_OFFSET,
        },
    },
    nao::CycleTime,
};

pub(super) struct WalkGaitPlugin;

impl Plugin for WalkGaitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WalkState>();
        app.add_systems(
            Update,
            (
                check_foot_switched,
                generate_foot_positions,
                update_swing_foot,
            )
                .chain()
                .in_set(MotionSet::GaitGeneration)
                .run_if(in_state(MotionState::Walking)),
        );
    }
}

#[derive(Debug, Clone, Resource)]
struct WalkState {
    phase: Duration,
    start: FootPositions,
    planned_duration: Duration,
    foot_switched_fsr: bool,
}

impl Default for WalkState {
    fn default() -> Self {
        Self {
            phase: Duration::ZERO,
            start: FootPositions::default(),
            planned_duration: Duration::from_millis(200),
            foot_switched_fsr: false,
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
        (self.phase.as_secs_f32() / self.planned_duration.as_secs_f32()).clamp(0.0, 1.0)
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

fn check_foot_switched(
    mut state: ResMut<WalkState>,
    swing_foot: Res<SwingFoot>,
    fsr: Res<ForceSensitiveResistors>,
    config: Res<WalkingEngineConfig>,
) {
    let left_foot_fsr = fsr.left_foot.sum();
    let right_foot_fsr = fsr.right_foot.sum();

    state.foot_switched_fsr = match **swing_foot {
        Side::Left => left_foot_fsr,
        Side::Right => right_foot_fsr,
    } > config.cop_pressure_threshold;
}

fn generate_foot_positions(
    mut state: ResMut<WalkState>,
    mut target_positions: ResMut<TargetFootPositions>,
    swing_foot: Res<SwingFoot>,
    cycle_time: Res<CycleTime>,
    config: Res<WalkingEngineConfig>,
    kinematics: Res<Kinematics>,
) {
    state.phase += cycle_time.duration;
    let linear = state.linear();
    let parabolic = state.parabolic();

    // TODO: replace with proper step planning
    let step = Step {
        forward: 0.00,
        left: 0.0,
        turn: -0.3,
        duration: state.planned_duration,
        swing_foot_height: 0.01,
        swing_foot: **swing_foot,
    }
    .clamp_anatomic(0.1);

    info!(
        ?swing_foot,
        "walking duration: {:?}, phase: {:?}", state.planned_duration, state.phase
    );

    let target = FootPositions::from_target(&step);
    let turn_travel = match step.swing_foot {
        Side::Left => target.left.rotation.angle_to(&state.start.left.rotation),
        Side::Right => target.right.rotation.angle_to(&state.start.right.rotation),
    };

    info!("turn_travel: {:.5}", turn_travel);

    let (left_t, right_t) = match &step.swing_foot {
        Side::Left => (parabolic, linear),
        Side::Right => (linear, parabolic),
    };

    let mut left = state.start.left.lerp_slerp(&target.left.inner, left_t);
    let mut right = state.start.right.lerp_slerp(&target.right.inner, right_t);

    let real = FootPositions::from_kinematics(**swing_foot, &kinematics, TORSO_OFFSET);

    info!(
        "[left] start: {:.4} target: {:.4}, current: {:.4} real: {:.4}",
        state.start.left.rotation.euler_angles().2,
        target.left.rotation.euler_angles().2,
        left.rotation.euler_angles().2,
        real.left.rotation.euler_angles().2
    );
    info!(
        "[right] start: {:.4} target: {:.4}, current: {:.4} real: {:.4}",
        state.start.right.rotation.euler_angles().2,
        target.right.rotation.euler_angles().2,
        right.rotation.euler_angles().2,
        real.right.rotation.euler_angles().2
    );

    let swing_lift = parabolic_return(linear) * compute_step_apex(&config, &step);
    let (left_lift, right_lift) = match &step.swing_foot {
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

/// System that switches the current swing foot when possible.
fn update_swing_foot(
    mut swing_foot: ResMut<SwingFoot>,
    mut state: ResMut<WalkState>,
    kinematics: Res<Kinematics>,
) {
    if !state.foot_switched_fsr || state.linear() <= 0.75 {
        return;
    }

    info!("Switching foot!");
    state.phase = Duration::ZERO;
    state.planned_duration = Duration::from_secs_f32(0.25);
    state.start = FootPositions::from_kinematics(swing_foot.opposite(), &kinematics, TORSO_OFFSET);
    **swing_foot = swing_foot.opposite();
    state.foot_switched_fsr = false;
}

fn compute_step_apex(config: &WalkingEngineConfig, step: &Step) -> f32 {
    step.swing_foot_height
        + config.foot_lift_modifier.forward * step.forward
        + config.foot_lift_modifier.left * step.left
        + config.foot_lift_modifier.turn * step.turn
}
