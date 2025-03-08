use bevy::prelude::*;
use std::time::Duration;

use crate::{
    kinematics::Kinematics,
    motion::walking_engine::{
        FootSwitchedEvent, Side, TargetFootPositions,
        config::{KickingConfig, WalkingEngineConfig},
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

pub(super) struct StoppingPlugin;

impl Plugin for StoppingPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Gait::Stopping), init_stopping_step);
        app.add_systems(
            PreUpdate,
            end_stopping_phase
                .after(crate::kinematics::update_kinematics)
                .before(step_context::sync_gait_request)
                .in_set(WalkingEngineSet::Prepare)
                .run_if(in_state(Gait::Stopping)),
        );
        app.add_systems(
            Update,
            generate_stopping_gait
                .in_set(WalkingEngineSet::GenerateGait)
                .run_if(in_state(Gait::Stopping)),
        );
    }
}

fn init_stopping_step(
    mut commands: Commands,
    mut step_context: ResMut<StepContext>,
    kinematics: Res<Kinematics>,
    walking_engine_config: Res<WalkingEngineConfig>,
    kicking_config: Res<KickingConfig>,
) {
    let start = FootPositions::from_kinematics(
        step_context.planned_step.swing_side,
        &kinematics,
        walking_engine_config.torso_offset,
    );
    step_context.plan_next_step(start, &walking_engine_config, &kicking_config);

    commands.insert_resource(WalkState {
        phase: Duration::ZERO,
        planned_step: PlannedStep {
            step: Step::default(),
            target: FootPositions::default(),
            swing_foot_height: walking_engine_config.stopping_foot_lift,
            duration: walking_engine_config.stopping_step_duration,
            ..step_context.planned_step
        },
    });
}

fn end_stopping_phase(
    mut step_context: ResMut<StepContext>,
    state: Res<WalkState>,
    mut foot_support: ResMut<FootSupportState>,
    mut event: EventWriter<FootSwitchedEvent>,
    config: Res<WalkingEngineConfig>,
) {
    let stopping_end_allowed = state.linear() > config.minimum_step_duration_ratio;
    let support_switched = foot_support.switched();
    let step_timeout = state.phase >= state.planned_step.duration;

    if (support_switched || step_timeout) && stopping_end_allowed {
        step_context.finish_stopping_step(state.planned_step);
        foot_support.switch_support_side();
        event.write(FootSwitchedEvent {
            new_support: foot_support.support_side(),
            new_swing: foot_support.swing_side(),
        });
    }
}

fn generate_stopping_gait(
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
