use bevy::prelude::*;
use nidhogg::types::Fsr;
use serde::{Deserialize, Serialize};

use crate::{
    prelude::*,
    sensor::fsr::{CalibratedFsr, Contacts},
};

use super::{config::WalkingEngineConfig, schedule::WalkingEngineSet, Side};

pub(super) struct FootSupportPlugin;

impl Plugin for FootSupportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FootSupportState>();
        app.add_systems(PostStartup, init_foot_support);
        app.add_systems(
            Sensor,
            update_foot_support
                .after(crate::sensor::fsr::update_contacts)
                .after(crate::sensor::fsr::update_fsr_calibration)
                .in_set(WalkingEngineSet::Prepare),
        );
    }
}

/// Configuration for the foot support plugin.
#[derive(Resource, Serialize, Deserialize, Debug, Default, Clone)]
#[serde(deny_unknown_fields)]
pub struct FootSupportConfig {
    /// The maximum (normalized) pressure on the current support foot, before predicting a foot switch.
    predict_support_max_pressure: f32,
    /// The minimum (normalized) pressure on the current swing foot, before predicting a foot switch.
    predict_swing_min_pressure: f32,
    /// The number of cycles to look in the future when predicting a support foot switch.
    predict_num_cycles: f32,
    /// The weights for the normalized FSR values, as contact with outer part of the foot is more important
    /// when deciding which foot is used as support foot.
    weights: Fsr,
}

/// Tracks the distribution of weight between feet during walking,
/// to keep track which foot is supporting the body of the robot.
#[derive(Resource, Debug, Default)]
pub struct FootSupportState {
    /// Current weight distribution between feet (-1 for left support, 1 for right support).
    support_ratio: f32,

    /// Previous cycle's weight distribution, used to detect changes in support.
    last_support_ratio: f32,

    /// Last recorded weight distribution with reliable pressure data from the supporting foot.
    last_support_ratio_with_pressure: f32,

    /// Indicates if the current support data is considered trusted based on fsr calibration.
    pub trusted: bool,

    /// Indicates if the support foot changed during this cycle.
    foot_switched: bool,

    /// Indicates if a foot switch is predicted in the next few cycles.
    predicted_switch: bool,

    /// The current supporting foot.
    ///
    /// This value is not explicitly decided by `support_ratio`, because gait generators
    /// might need more explicit control over which side is the support side.
    support: Side,
}

impl FootSupportState {
    /// Get the current support side.
    #[must_use]
    pub fn support_side(&self) -> Side {
        self.support
    }

    /// Get the current swing side.
    #[must_use]
    pub fn swing_side(&self) -> Side {
        self.support.opposite()
    }

    /// Switch the support foot side.
    pub(super) fn switch_support_side(&mut self) {
        self.support = self.support.opposite();
    }

    /// Return `true` if a foot switch has occurred or is predicted in future cycles.
    #[must_use]
    pub fn predicted_or_switched(&self) -> bool {
        // we only trust prediction if fsr is calibrated
        (self.trusted && self.predicted_switch) || self.foot_switched
    }

    /// Return `true` if a foot switch has occurred in the current cycle.
    #[must_use]
    pub fn switched(&self) -> bool {
        self.foot_switched
    }
}

fn init_foot_support(mut commands: Commands, config: Res<WalkingEngineConfig>) {
    commands.insert_resource(config.foot_support.clone());
}

fn update_foot_support(
    config: Res<FootSupportConfig>,
    mut state: ResMut<FootSupportState>,
    calibration: Res<CalibratedFsr>,
    contacts: Res<Contacts>,
) {
    let pressures = &calibration.normalized;
    let weighted_pressure = pressures.weighted_sum(&config.weights);
    let total_pressure = pressures
        .left_foot
        .weighted_sum(&config.weights.left_foot)
        .abs()
        + pressures
            .right_foot
            .weighted_sum(&config.weights.right_foot)
            .abs();

    if total_pressure > 0.0 {
        state.trusted = true;
        state.support_ratio = weighted_pressure / total_pressure;

        let support_has_pressure = (contacts.left_foot && state.support_ratio > 0.0)
            || (contacts.right_foot && state.support_ratio < 0.0);
        let switched = (state.last_support_ratio_with_pressure * state.support_ratio < 0.0
            || (state.last_support_ratio == 0.0 && state.support_ratio != 0.0))
            && support_has_pressure;

        if support_has_pressure {
            state.last_support_ratio_with_pressure = state.support_ratio;
        }

        state.foot_switched = switched;
        let predicted_support = state.support_ratio
            + config.predict_num_cycles * (state.support_ratio - state.last_support_ratio);

        let left_support_can_predict = state.support_ratio < 0.0
            && pressures.right_foot.avg() < config.predict_support_max_pressure
            && pressures.left_foot.avg() > config.predict_swing_min_pressure;
        let right_support_can_predict = state.support_ratio > 0.0
            && pressures.left_foot.avg() < config.predict_support_max_pressure
            && pressures.right_foot.avg() > config.predict_swing_min_pressure;

        let predicted_switch = predicted_support * state.support_ratio < 0.
            && (left_support_can_predict || right_support_can_predict);

        state.last_support_ratio = state.support_ratio;

        // Only predict a foot switch if the FSR is calibrated
        if calibration.is_calibrated {
            state.predicted_switch = predicted_switch;
        }
    }
}
