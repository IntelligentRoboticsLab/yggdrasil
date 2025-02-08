use bevy::prelude::*;
use nidhogg::types::Fsr;
use serde::{Deserialize, Serialize};

use crate::sensor::fsr::{CalibratedFsr, Contacts};

use super::{
    config::WalkingEngineConfig,
    schedule::{MotionSet, StepPlanning},
    Side,
};

pub(super) struct FootSupportPlugin;

impl Plugin for FootSupportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FootSupportState>();
        app.add_systems(PostStartup, init_foot_support);
        app.add_systems(
            StepPlanning,
            update_foot_support.in_set(MotionSet::StepPlanning),
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

#[derive(Resource, Debug, Default)]
pub struct FootSupportState {
    support: f32,
    last_support: f32,
    last_support_with_pressure: f32,
    trusted: bool,
    pub foot_switched: bool,
    pub predicted_switch: bool,
}

impl FootSupportState {
    /// Get the current support foot.
    pub fn current_support(&self) -> Side {
        if self.support < 0.0 {
            Side::Left
        } else {
            Side::Right
        }
    }

    /// Get the current swing foot.
    pub fn current_swing(&self) -> Side {
        self.current_support().opposite()
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
        state.support = weighted_pressure / total_pressure;

        let support_has_pressure = (contacts.left_foot && state.support > 0.0)
            || (contacts.right_foot && state.support < 0.0);
        let switched = (state.last_support_with_pressure * state.support < 0.0
            || (state.last_support == 0.0 && state.support != 0.0))
            && support_has_pressure;

        if support_has_pressure {
            state.last_support_with_pressure = state.support;
        }

        state.foot_switched = switched;
        let predicted_support =
            state.support + config.predict_num_cycles * (state.support - state.last_support);

        let left_support_can_predict = state.support < 0.0
            && pressures.right_foot.avg() < config.predict_support_max_pressure
            && pressures.left_foot.avg() > config.predict_swing_min_pressure;
        let right_support_can_predict = state.support > 0.0
            && pressures.left_foot.avg() < config.predict_support_max_pressure
            && pressures.right_foot.avg() > config.predict_swing_min_pressure;

        let predicted_switch = predicted_support * state.support < 0.
            && (left_support_can_predict || right_support_can_predict);

        state.last_support = state.support;

        // Only predict a foot switch if the FSR is calibrated
        if calibration.is_calibrated {
            state.predicted_switch = predicted_switch;
        }
    }
}
