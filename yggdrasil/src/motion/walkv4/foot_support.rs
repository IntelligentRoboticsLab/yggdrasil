use bevy::prelude::*;
use nidhogg::types::{ForceSensitiveResistorFoot, ForceSensitiveResistors};

use crate::{core::debug::DebugContext, sensor::fsr::FsrCalibration};

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

fn update_foot_support(
    dbg: DebugContext,
    mut state: ResMut<FootSupportState>,
    fsr: Res<ForceSensitiveResistors>,
    calibration: Res<FsrCalibration>,
    swing_foot: Res<SwingFoot>,
) {
    let weighted_pressure = fsr.weighted_sum(&FSR_WEIGHTS);
    let total_pressure = fsr.sum();

    if total_pressure > 0.0 {
        state.trusted = true;
        state.support = weighted_pressure / total_pressure;

        // TODO: make sure new support foot has pressure.
        let switched = (state.last_support_with_pressure * state.support < 0.0
            || (state.last_support_with_pressure == 0.0 && state.support != 0.0));

        state.last_support_with_pressure = match (**swing_foot, state.support > 0.) {
            (Side::Left, true) => state.support,
            (Side::Left, false) => state.last_support_with_pressure,
            (Side::Right, true) => state.last_support_with_pressure,
            (Side::Right, false) => state.support,
        };

        if switched {
            info!("switched normally?!");
        }

        //float predictedSupport = theFootSupport.support + 3.f * (theFootSupport.support - lastSupport); //current vel
        let predicted_support = state.support + 3.0 * (state.support - state.last_support);
        info!(
            "support: {:.3}, predicted: {:.3}",
            state.support, predicted_support
        );

        state.last_support = state.support;
    }
}
