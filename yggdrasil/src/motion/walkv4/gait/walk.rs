use std::time::{Duration, Instant};

use bevy::prelude::*;
use nalgebra::Vector2;
use nidhogg::types::Fsr;

use crate::{
    kinematics::Kinematics,
    motion::{
        walk::smoothing::{parabolic_return, parabolic_step},
        walkv4::{
            config::{ConfigStep, WalkingEngineConfig},
            feet::FootPositions,
            foot_support::FootSupportState,
            scheduling::{MotionSet, MotionState},
            step::Step,
            FootSwitchedEvent, RequestedStep, Side, SwingFoot, TargetFootPositions, TORSO_OFFSET,
        },
    },
    nao::{Cycle, CycleTime},
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
    last_step: ConfigStep,
}

impl Default for WalkState {
    fn default() -> Self {
        Self {
            phase: Duration::ZERO,
            start: FootPositions::default(),
            planned_duration: Duration::from_millis(200),
            foot_switched_fsr: false,
            last_step: ConfigStep::default(),
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
    foot_support: Res<FootSupportState>,
    cycle: Res<Cycle>,
    mut last_switched: Local<Cycle>,
) {
    let switch_diff = cycle.0 - last_switched.0;
    // only switch if we've completed 75% of the step
    let is_switch_allowed = state.linear() > 0.75;

    if foot_support.predicted_switch {
        println!("switch diff: {switch_diff}");
        if switch_diff >= 20 && is_switch_allowed {
            println!("predicted switch!");
            state.foot_switched_fsr = true;
        }
    } else {
        if switch_diff >= 20 && is_switch_allowed {
            state.foot_switched_fsr = foot_support.foot_switched;
        }
    }

    if state.foot_switched_fsr {
        *last_switched = *cycle;
    }
}

fn generate_foot_positions(
    mut state: ResMut<WalkState>,
    mut target_positions: ResMut<TargetFootPositions>,
    swing_foot: Res<SwingFoot>,
    cycle_time: Res<CycleTime>,
    config: Res<WalkingEngineConfig>,
    requested_step: Res<RequestedStep>,
) {
    state.phase += cycle_time.duration;
    let linear = state.linear();
    let parabolic = state.parabolic();

    // TODO: replace with proper step planning
    let mut step = Step {
        forward: requested_step.forward,
        left: requested_step.left,
        turn: requested_step.turn,
        duration: state.planned_duration,
        swing_foot_height: 0.01,
        swing_foot: **swing_foot,
    }
    .clamp_anatomic(0.1);

    let target = FootPositions::from_target(&step);

    let turn_travel = match &step.swing_foot {
        Side::Left => target
            .left
            .inner
            .rotation
            .angle_to(&state.start.left.inner.rotation),
        Side::Right => target
            .right
            .inner
            .rotation
            .angle_to(&state.start.right.inner.rotation),
    };

    let swing_travel = state
        .start
        .swing_travel_over_ground(step.swing_foot, &target)
        .abs();

    let foot_lift_apex = config.base_foot_lift
        + travel_weighting(swing_travel, turn_travel, config.foot_lift_modifier);

    step.swing_foot_height = foot_lift_apex;

    let (left_t, right_t) = match &step.swing_foot {
        Side::Left => (parabolic, linear),
        Side::Right => (linear, parabolic),
    };

    let mut left = state.start.left.lerp_slerp(&target.left.inner, left_t);
    let mut right = state.start.right.lerp_slerp(&target.right.inner, right_t);

    let swing_lift = parabolic_return(linear) * foot_lift_apex;
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

fn travel_weighting(
    translation_travel: Vector2<f32>,
    turn_travel: f32,
    factors: ConfigStep,
) -> f32 {
    let translational = nalgebra::vector![
        factors.forward * translation_travel.x,
        factors.left * translation_travel.y,
    ]
    .norm();
    let rotational = factors.turn * turn_travel;
    translational + rotational
}

/// System that switches the current swing foot when possible.
fn update_swing_foot(
    mut event: EventWriter<FootSwitchedEvent>,
    mut swing_foot: ResMut<SwingFoot>,
    mut state: ResMut<WalkState>,
    kinematics: Res<Kinematics>,
    requested_step: Res<RequestedStep>,
) {
    if !state.foot_switched_fsr {
        return;
    }

    info!("\nSwitching foot!\n");
    state.phase = Duration::ZERO;
    state.planned_duration = Duration::from_secs_f32(0.25);
    state.start = FootPositions::from_kinematics(swing_foot.opposite(), &kinematics, TORSO_OFFSET);
    **swing_foot = swing_foot.opposite();
    state.foot_switched_fsr = false;
    state.last_step = ConfigStep {
        forward: requested_step.forward,
        left: requested_step.left,
        turn: requested_step.turn,
    };
    event.send(FootSwitchedEvent(**swing_foot));
}
