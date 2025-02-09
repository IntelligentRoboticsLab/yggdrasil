use bevy::prelude::*;
use std::time::Duration;

use crate::{
    kinematics::Kinematics,
    motion::walkv4::{
        config::WalkingEngineConfig,
        feet::FootPositions,
        foot_support::FootSupportState,
        schedule::{Gait, GaitGeneration, MotionSet, StepPlanning},
        smoothing::{parabolic_return, parabolic_step},
        step::{PlannedStep, Step},
        step_manager::StepManager,
        Side, TargetFootPositions, TORSO_OFFSET,
    },
    nao::CycleTime,
};
pub(super) struct StartingPlugin;

impl Plugin for StartingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Gait::Starting), init_starting_step);
        app.add_systems(
            StepPlanning,
            end_starting_phase
                .in_set(MotionSet::StepPlanning)
                .run_if(in_state(Gait::Starting)),
        );
        app.add_systems(
            GaitGeneration,
            generate_starting_gait
                .in_set(MotionSet::GaitGeneration)
                .run_if(in_state(Gait::Starting)),
        );
    }
}

#[derive(Resource, Debug)]
struct StartingState {
    phase: Duration,
    planned_step: PlannedStep,
}

impl Default for StartingState {
    fn default() -> Self {
        Self {
            phase: Duration::ZERO,
            planned_step: PlannedStep::default(),
        }
    }
}

impl StartingState {
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

fn init_starting_step(
    mut commands: Commands,
    mut step_manager: ResMut<StepManager>,
    kinematics: Res<Kinematics>,
    config: Res<WalkingEngineConfig>,
) {
    step_manager.plan_next_step(
        FootPositions::from_kinematics(Side::Left, &kinematics, TORSO_OFFSET),
        &config,
    );
    commands.insert_resource(StartingState {
        phase: Duration::ZERO,
        planned_step: PlannedStep {
            step: Step::default(),
            target: FootPositions::default(),
            swing_foot_height: 0.009,
            duration: Duration::from_millis(200),
            ..step_manager.planned_step
        },
    });
}

fn end_starting_phase(
    mut step_manager: ResMut<StepManager>,
    state: Res<StartingState>,
    foot_support: ResMut<FootSupportState>,
) {
    let starting_end_allowed = state.linear() > 0.75;
    let support_switched = foot_support.foot_switched;
    let step_timeout = state.phase >= state.planned_step.duration;

    if (support_switched || step_timeout) && starting_end_allowed {
        step_manager.finish_starting_step(state.planned_step);
        info!("finished starting state!");
    }
}

fn generate_starting_gait(
    mut state: ResMut<StartingState>,
    mut target_positions: ResMut<TargetFootPositions>,

    cycle_time: Res<CycleTime>,
) {
    state.phase += cycle_time.duration;
    let linear = state.linear();
    let parabolic = state.parabolic();

    let planned = state.planned_step;
    let (left_t, right_t) = match &planned.swing_foot {
        Side::Left => (parabolic, linear),
        Side::Right => (linear, parabolic),
    };

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

    info!(?left_lift, ?right_lift, "starting step!");

    **target_positions = FootPositions {
        left: left.into(),
        right: right.into(),
    };
}
