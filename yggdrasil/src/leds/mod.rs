use crate::{
    nao::arbiter::{NaoArbiter, Priority},
    prelude::*,
};

use std::time::{Duration, Instant};

pub use nidhogg::types::{
    color::{self, RgbF32},
    FillExt, LeftEar, LeftEye, RightEar, RightEye, Skull,
};

use crate::{behavior, nao};

/// A module providing functionality to manipulate the colors of various LEDS
/// on the NAO robot.
///
/// This module provides the following resources to the application:
/// - [`Leds`]
pub struct LedsModule;

impl Module for LedsModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_resource(Resource::new(Leds::default()))?
            .add_system(
                write_led_values
                    .before(behavior::engine::step)
                    .after(nao::write_hardware_info),
            ))
    }
}

#[derive(Default)]
pub struct Leds {
    /// LEDs in the left ear
    pub left_ear: LeftEar,
    /// LEDs in the right ear
    pub right_ear: RightEar,
    /// LED in the chest
    pub chest: RgbF32,
    /// LEDs in the left eye
    pub left_eye: LeftEye,
    /// LEDs in the right eye
    pub right_eye: RightEye,
    /// LED in the left foot
    pub left_foot: RgbF32,
    /// LED in the right foot
    pub right_foot: RgbF32,
    /// LEDs on the head
    pub skull: Skull,
    /// Keeps track of information for letting the chest LED blink.
    chest_blink: Option<ChestBlink>,
    // States that are used for more complicated patterns can be added below.
}

#[derive(Clone)]
struct ChestBlink {
    color: RgbF32,
    interval: Duration,
    on: bool,
    start: Instant,
}

impl Leds {
    /// Makes the LED in the chest blink a given color with a given interval.
    pub fn set_chest_blink(&mut self, color: RgbF32, interval: Duration) {
        match &mut self.chest_blink {
            Some(blink) => {
                // We can just update the existing blink
                blink.color = color;
                blink.interval = interval;
            }
            None => {
                self.chest_blink = Some(ChestBlink {
                    color,
                    interval,
                    on: true,
                    start: Instant::now(),
                });
            }
        }
    }

    pub fn unset_chest_blink(&mut self) {
        self.chest_blink = None;
    }
}

#[system]
pub fn write_led_values(leds: &mut Leds, nao_arbiter: &mut NaoArbiter) -> Result<()> {
    nao_arbiter
        .set_chest_led(leds.chest, Priority::High)
        .set_left_foot_led(leds.left_foot, Priority::High)
        .set_right_foot_led(leds.right_foot, Priority::High)
        .set_left_ear_led(leds.left_ear.clone(), Priority::High)
        .set_right_ear_led(leds.right_ear.clone(), Priority::High)
        .set_left_eye_led(leds.left_eye.clone(), Priority::High)
        .set_right_eye_led(leds.right_eye.clone(), Priority::High)
        .set_skull_led(leds.skull.clone(), Priority::High);

    if let Some(blink) = leds.chest_blink.as_mut() {
        if blink.start.elapsed() > blink.interval {
            blink.on = !blink.on;
            blink.start = Instant::now();
        }

        if blink.on {
            nao_arbiter.set_chest_led(blink.color, Priority::High);
        } else {
            nao_arbiter.set_chest_led(color::f32::EMPTY, Priority::High);
        }
    }

    Ok(())
}
