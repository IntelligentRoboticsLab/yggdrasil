use bevy::prelude::*;
use std::time::Duration;
use tracing::info;

use crate::{
    kinematics::Kinematics,
    motion::walking_engine::{
        FootSwitchedEvent, Side, TargetFootPositions,
        config::WalkingEngineConfig,
        feet::FootPositions,
        foot_support::FootSupportState,
        schedule::{Gait, WalkingEngineSet},
        smoothing::parabolic_return,
        step::{PlannedStep, Step},
        step_context::{self, StepContext},
    },
    nao::CycleTime,
};

use super::WalkState;
pub(super) struct StartingPlugin;

impl Plugin for StartingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Gait::Starting), init_starting_step);
        app.add_systems(
            PreUpdate,
            end_starting_phase
                .after(crate::kinematics::update_kinematics)
                .before(step_context::sync_gait_request)
                .in_set(WalkingEngineSet::Prepare)
                .run_if(in_state(Gait::Starting)),
        );
        app.add_systems(
            Update,
            generate_starting_gait
                .in_set(WalkingEngineSet::GenerateGait)
                .run_if(in_state(Gait::Starting)),
        );
    }
}

fn init_starting_step(
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
    info!("initializing starting step...");
    step_context.plan_next_step(start, &config);

    commands.insert_resource(WalkState {
        phase: Duration::ZERO,
        planned_step: PlannedStep {
            step: Step::default(),
            target: FootPositions::default(),
            swing_foot_height: config.starting_foot_lift,
            duration: config.starting_step_duration,
            ..step_context.planned_step
        },
    });
}

fn end_starting_phase(
    mut step_context: ResMut<StepContext>,
    state: Res<WalkState>,
    mut foot_support: ResMut<FootSupportState>,
    mut event: EventWriter<FootSwitchedEvent>,
    config: Res<WalkingEngineConfig>,
) {
    let starting_end_allowed = state.linear() > config.minimum_step_duration_ratio;
    let support_switched = foot_support.switched();
    let step_timeout = state.phase >= state.planned_step.duration;

    if (support_switched || step_timeout) && starting_end_allowed {
        step_context.finish_starting_step(state.planned_step);
        foot_support.switch_support_side();
        event.write(FootSwitchedEvent {
            new_support: foot_support.support_side(),
            new_swing: foot_support.swing_side(),
        });
    }
}

fn generate_starting_gait(
    mut state: ResMut<WalkState>,
    mut target_positions: ResMut<TargetFootPositions>,
    cycle_time: Res<CycleTime>,
) {
    state.phase += cycle_time.duration;
    let linear = state.linear();
    let parabolic = state.parabolic();

    let planned = state.planned_step;
    let (left_t, right_t) = match &planned.swing_side {
        Side::Left => (parabolic, linear),
        Side::Right => (linear, parabolic),
    };

    let start = planned.start;
    let target = planned.target;
    let mut left = start.left.lerp_slerp(&target.left.inner, left_t);
    let mut right = start.right.lerp_slerp(&target.right.inner, right_t);

    let swing_lift = parabolic_return(linear) * planned.swing_foot_height;
    let (left_lift, right_lift) = match &planned.swing_side {
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
