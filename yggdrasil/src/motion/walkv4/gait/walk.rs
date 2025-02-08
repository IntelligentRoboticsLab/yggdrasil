use std::time::Duration;

use bevy::prelude::*;

use crate::{
    motion::walkv4::{
        feet::FootPositions,
        foot_support::FootSupportState,
        scheduling::{Gait, MotionSet},
        smoothing::{parabolic_return, parabolic_step},
        step::PlannedStep,
        step_manager::StepManager,
        FootSwitchedEvent, Side, SwingFoot, TargetFootPositions,
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
                .run_if(in_state(Gait::Walking)),
        );
    }
}

#[derive(Debug, Clone, Resource)]
struct WalkState {
    phase: Duration,
    planned_step: PlannedStep,
    foot_switched_fsr: bool,
}

impl Default for WalkState {
    fn default() -> Self {
        Self {
            phase: Duration::ZERO,
            planned_step: PlannedStep::default(),
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
    cycle_time: Res<CycleTime>,
    step_manager: Res<StepManager>,
) {
    state.phase += cycle_time.duration;
    let linear = state.linear();
    let parabolic = state.parabolic();

    let (left_t, right_t) = match &step_manager.planned_step.swing_foot {
        Side::Left => (parabolic, linear),
        Side::Right => (linear, parabolic),
    };

    let planned = step_manager.planned_step;
    state.planned_step = planned;
    let start = planned.start;
    let target = planned.target;
    let mut left = start.left.lerp_slerp(&target.left.inner, left_t);
    let mut right = start.right.lerp_slerp(&target.right.inner, right_t);

    let swing_lift = parabolic_return(linear) * planned.swing_foot_height;
    let (left_lift, right_lift) = match &planned.swing_foot {
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
    mut event: EventWriter<FootSwitchedEvent>,
    mut swing_foot: ResMut<SwingFoot>,
    mut state: ResMut<WalkState>,
) {
    if !state.foot_switched_fsr {
        return;
    }

    info!("\nSwitching foot!\n");
    state.phase = Duration::ZERO;
    **swing_foot = swing_foot.opposite();
    state.foot_switched_fsr = false;
    event.send(FootSwitchedEvent(**swing_foot));
}
