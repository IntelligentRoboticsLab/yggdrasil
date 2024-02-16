use crate::prelude::*;

use std::time::{Duration, Instant};

use nidhogg::NaoControlMessage;

pub use nidhogg::types::{Color, FillExt, LeftEar, LeftEye, RightEar, RightEye, Skull};

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
    pub chest: Color,
    /// LEDs in the left eye
    pub left_eye: LeftEye,
    /// LEDs in the right eye
    pub right_eye: RightEye,
    /// LED in the left foot
    pub left_foot: Color,
    /// LED in the right foot
    pub right_foot: Color,
    /// LEDs on the head
    pub skull: Skull,
    /// Keeps track of information for letting the chest LED blink.
    chest_blink: Option<ChestBlink>,
    // States that are used for more complicated patterns can be added below.
}

#[derive(Clone)]
struct ChestBlink {
    color: Color,
    interval: Duration,
    on: bool,
    start: Instant,
}

impl Leds {
    /// Makes the LED in the chest blink a given color with a given interval.
    pub fn set_chest_blink(&mut self, color: Color, interval: Duration) {
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
pub fn write_led_values(leds: &mut Leds, control_message: &mut NaoControlMessage) -> Result<()> {
    control_message.chest = leds.chest;
    control_message.left_foot = leds.left_foot;
    control_message.right_foot = leds.right_foot;

    control_message.left_ear = leds.left_ear.clone();
    control_message.right_ear = leds.right_ear.clone();
    control_message.left_eye = leds.left_eye.clone();
    control_message.right_eye = leds.right_eye.clone();
    control_message.skull = leds.skull.clone();

    if let Some(blink) = leds.chest_blink.as_mut() {
        if blink.start.elapsed() > blink.interval {
            blink.on = !blink.on;
            blink.start = Instant::now();
        }

        if blink.on {
            control_message.chest = blink.color;
        } else {
            control_message.chest = Color::EMPTY;
        }
    }

    Ok(())
}
