use std::collections::VecDeque;

use crate::{nao::manager::{NaoManager, Priority}, prelude::*};
use nidhogg::{types::{color, FillExt, RightEye, SonarValues}, NaoState};

/// A module offering structured wrappers for sonar, derived from the raw [`NaoState`].
///
/// This module provides the following resources to the application:
/// - [`SonarValues`]
pub struct SonarSensor;

impl Module for SonarSensor {
    fn initialize(self, app: App) -> Result<App> {
        app.add_staged_system(SystemStage::Sensor, sonar_sensor)
            .init_resource::<Sonar>()
    }
}

#[system]
fn sonar_sensor(nao_state: &NaoState, nao: &mut NaoManager, sonar: &mut Sonar) -> Result<()> {
    sonar.update_from_values(&nao_state.sonar);

    if sonar.obstructed() {
        nao.set_right_eye_led(RightEye::fill(color::f32::RED), Priority::High);
    } else {
        nao.set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::High);
    }

    Ok(())
}

#[derive(Debug, Default)]
pub struct Sonar {
    left: SonarSide,
    right: SonarSide,
    obstructed: bool,
}

#[derive(Debug, Default)]
struct SonarSide {
    accum: (usize, f32),
    history: VecDeque<Option<f32>>,
}

impl Sonar {
    pub fn obstructed(&self) -> bool {
        self.obstructed
    }

    fn is_obstructed(&self) -> bool {
        let left = self.left.ratio();
        let right = self.right.ratio();
        let both = left + right;

        if left > 0.3 && right > 0.3 {
            both > 0.3 && self.left.mean().min(self.right.mean()) < 0.4
        } else {
            both > 0.8 && self.left.mean().min(self.right.mean()) < 0.2
        }

    }

    fn update_from_values(&mut self, values: &SonarValues) {
        self.left.update_from_value(values.left);
        self.right.update_from_value(values.right);
        self.obstructed = self.is_obstructed();
    }

}

impl SonarSide {
    const HISTORY_SIZE: usize = 128;

    fn ratio(&self) -> f32 {
        self.accum.0 as f32 / Self::HISTORY_SIZE as f32
    }

    fn mean(&self) -> f32 {
        self.accum.1 / self.accum.0 as f32
    }

    fn update_from_value(&mut self, value: f32) {
        let value = (value < 5.).then_some(value); 

        if let Some(value) = value {
            self.accum.0 += 1;
            self.accum.1 += value;
        }

        if self.history.len() >= Self::HISTORY_SIZE {
            if let Some(old) = self.history.pop_front().unwrap() {
                self.accum.0 -= 1;

                if self.accum.0 == 0 {
                    self.accum.1 = 0.;
                } else {
                    self.accum.1 -= old;
                }
            }
        }

        self.history.push_back(value);
    }
}
