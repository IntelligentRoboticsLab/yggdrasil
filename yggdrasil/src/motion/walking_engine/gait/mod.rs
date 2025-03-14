use std::time::Duration;

use bevy::prelude::*;

use super::{
    FootSwitchedEvent, Gait, WalkingEngineSet, config::WalkingEngineConfig,
    foot_support::FootSupportState, smoothing::parabolic_step, step::PlannedStep,
};
use crate::prelude::*;

mod kick;
mod sit;
mod stand;
mod starting;
mod stopping;
pub mod walk;

pub use stand::StandingHeight;

pub(super) struct GaitPlugin;

impl Plugin for GaitPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WalkState>();
        app.add_systems(
            Sensor,
            update_support_foot
                .after(crate::sensor::fsr::update_force_sensitive_resistor_sensor)
                .after(WalkingEngineSet::Prepare)
                .run_if(in_state(Gait::Walking).or(in_state(Gait::Stopping))),
        );

        app.add_plugins((
            sit::SitGaitPlugin,
            stand::StandGaitPlugin,
            starting::StartingPlugin,
            walk::WalkPlugin,
            kick::KickPlugin,
            stopping::StoppingPlugin,
        ));
    }
}

#[derive(Debug, Clone, Resource)]
pub(super) struct WalkState {
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

        event.write(FootSwitchedEvent {
            new_support: foot_support.support_side(),
            new_swing: foot_support.swing_side(),
        });
    }
}
