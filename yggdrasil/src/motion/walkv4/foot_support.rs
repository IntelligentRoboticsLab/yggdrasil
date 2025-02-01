use bevy::prelude::*;
use nidhogg::types::{ForceSensitiveResistorFoot, ForceSensitiveResistors};

use crate::{
    core::debug::DebugContext,
    sensor::{
        fsr::{Contacts, FsrCalibration},
        SensorConfig,
    },
};

use super::{scheduling::MotionSet, Side, SwingFoot};

pub(super) struct FootSupportPlugin;

impl Plugin for FootSupportPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FootSupportState>();
        app.add_systems(Update, update_foot_support.in_set(MotionSet::StepPlanning));
    }
}

const FSR_WEIGHTS: ForceSensitiveResistors = ForceSensitiveResistors {
    left_foot: ForceSensitiveResistorFoot {
        front_left: 0.8,
        front_right: 0.3,
        rear_left: 0.8,
        rear_right: 0.3,
    },
    right_foot: ForceSensitiveResistorFoot {
        front_left: -0.8,
        front_right: -0.3,
        rear_left: -0.8,
        rear_right: -0.3,
    },
};

#[derive(Resource, Debug, Default)]
pub struct FootSupportState {
    current_support: Side,
    support: f32,
    last_support: f32,
    last_support_with_pressure: f32,
    trusted: bool,
}

const CURRENT_SUPPORT_MAX_PRESSURE: f32 = 0.36;
const CURRENT_SWING_MAX_PRESSURE: f32 = 0.1;

fn update_foot_support(
    dbg: DebugContext,
    mut state: ResMut<FootSupportState>,
    fsr: Res<ForceSensitiveResistors>,
    calibration: Res<FsrCalibration>,
    contacts: Res<Contacts>,
    config: Res<SensorConfig>,
) {
    let weighted_pressure = fsr.weighted_sum(&FSR_WEIGHTS);
    let total_pressure = fsr.left_foot.weighted_sum(&FSR_WEIGHTS.left_foot).abs()
        + fsr.right_foot.weighted_sum(&FSR_WEIGHTS.right_foot).abs();

    if total_pressure > 0.0 {
        state.trusted = true;
        state.support = weighted_pressure / total_pressure;

        // TODO: make sure new support foot has pressure.
        let support_has_pressure = (contacts.left_foot && state.support > 0.0)
            || (contacts.right_foot && state.support < 0.0);
        let switched = (state.last_support_with_pressure * state.support < 0.0
            || (state.last_support == 0.0 && state.support != 0.0))
            && support_has_pressure;

        if support_has_pressure {
            state.last_support_with_pressure = state.support;
        }

        if switched {
            info!("switched normally?!");
        }

        let predicted_support = state.support + 3.0 * (state.support - state.last_support);

        let pressures = calibration.normalized_foot_pressure(&fsr);
        println!(
            "fsr_left: {:.3}, fsr_right: {:.3}",
            pressures.left_foot.sum(),
            pressures.right_foot.sum()
        );
        let left_support_can_predict = state.support < 0.0
            && pressures.right_foot.sum() < CURRENT_SUPPORT_MAX_PRESSURE
            && pressures.left_foot.sum() > CURRENT_SWING_MAX_PRESSURE;
        let right_support_can_predict = state.support > 0.0
            && pressures.left_foot.sum() < CURRENT_SUPPORT_MAX_PRESSURE
            && pressures.right_foot.sum() > CURRENT_SWING_MAX_PRESSURE;

        let predicted_switch = predicted_support * state.support < 0.
            && (left_support_can_predict || right_support_can_predict);

        println!(
            "support: {:.3}, predicted: {:.3}, predicted_switch: {} left: {}, right: {}",
            state.support,
            predicted_support,
            predicted_switch,
            left_support_can_predict,
            right_support_can_predict,
        );

        state.last_support = state.support;
    }
}
