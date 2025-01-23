use std::time::Duration;

use bevy::prelude::*;
use nidhogg::types::ForceSensitiveResistors;

use crate::{
    kinematics::{
        self,
        prelude::{ROBOT_TO_LEFT_PELVIS, ROBOT_TO_RIGHT_PELVIS},
        FootOffset, Kinematics,
    },
    motion::{
        walk::{
            smoothing::{parabolic_return, parabolic_step},
            WalkingEngineConfig,
        },
        walkv4::{
            feet::FootPositions,
            scheduling::{MotionSet, MotionState},
            step::Step,
            Side, SwingFoot, TargetFootPositions,
        },
    },
    nao::CycleTime,
    sensor::low_pass_filter::LowPassFilter,
};

// TODO: dynamically set this
/// The offset of the torso w.r.t. the hips.
const TORSO_OFFSET: f32 = 0.025;

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
    filtered_gyro: LowPassFilter<3>,
    foot_switched_fsr: bool,
}

impl Default for WalkState {
    fn default() -> Self {
        Self {
            phase: Duration::ZERO,
            start: FootPositions::default(),
            planned_duration: Duration::ZERO,
            filtered_gyro: LowPassFilter::new(0.3),
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
    swing_foot: Res<SwingFoot>,
    mut target: ResMut<TargetFootPositions>,
    cycle_time: Res<CycleTime>,
    config: Res<WalkingEngineConfig>,
) {
    state.phase += cycle_time.duration;
    let linear = state.linear();
    let parabolic = state.parabolic();

    // TODO: replace with proper step planning
    let step = Step {
        forward: 0.05,
        left: 0.0,
        turn: 0.0,
        duration: state.planned_duration,
        swing_foot_height: 0.01,
        swing_foot: **swing_foot,
    };

    let target = FootPositions::from_target(&step);

    let (left_t, right_t) = match &step.swing_foot {
        Side::Left => (parabolic, linear),
        Side::Right => (linear, parabolic),
    };

    let left = state.start.left.lerp_slerp(&target.left.inner, left_t);
    let right = state.start.right.lerp_slerp(&target.right.inner, right_t);

    let swing_lift = parabolic_return(linear) * compute_step_apex(&config, &step);
    let (left_lift, right_lift) = match &step.swing_foot {
        Side::Left => (swing_lift, 0.),
        Side::Right => (0., swing_lift),
    };

    let left_foot_offset = FootOffset {
        forward: left.translation.x,
        left: left.translation.y - ROBOT_TO_LEFT_PELVIS.y,
        turn: 0.,
        lift: left_lift,
        hip_height,
        ..Default::default()
    };

    let right_foot_offset = FootOffset {
        forward: right.translation.x,
        left: right.translation.y - ROBOT_TO_RIGHT_PELVIS.y,
        turn: 0.,
        lift: right_lift,
        hip_height,
        ..Default::default()
    };

    let (mut left, mut right) =
        kinematics::inverse::leg_angles(&left_foot_offset, &right_foot_offset, TORSO_OFFSET);

    // always set the foot offsets to 0,0,0.
    // **target = FootPositions::default();
}

/// System that switches the current swing foot when possible.
fn update_swing_foot(
    mut swing_foot: ResMut<SwingFoot>,
    mut state: ResMut<WalkState>,
    kinematics: Res<Kinematics>,
) {
    if !state.foot_switched_fsr && state.linear() <= 0.75 {
        return;
    }

    state.phase = Duration::ZERO;
    state.planned_duration = Duration::from_secs_f32(0.25);
    state.start = FootPositions::from_kinematics(swing_foot.opposite(), &kinematics, TORSO_OFFSET);
    **swing_foot = swing_foot.opposite();
}

fn compute_step_apex(config: &WalkingEngineConfig, step: &Step) -> f32 {
    step.swing_foot_height
        + config.foot_lift_modifier.forward * step.forward
        + config.foot_lift_modifier.left * step.left
        + config.foot_lift_modifier.turn * step.turn
}
