use std::time::{Duration, Instant};

use miette::Result;
use nidhogg::NaoControlMessage;
use tyr::prelude::*;

pub use nidhogg::types::{Color, FillExt, LeftEar, LeftEye, RightEar, RightEye, Skull};

pub struct LedsModule;

impl Module for LedsModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_resource(Resource::new(Led::default()))?
            .add_system(write_led_values))
    }
}

#[derive(Default)]
pub struct Led {
    pub left_ear: LeftEar,
    pub right_ear: RightEar,
    pub chest: Color,
    pub left_eye: LeftEye,
    pub right_eye: RightEye,
    pub left_foot: Color,
    pub right_foot: Color,
    pub skull: Skull,

    /// States that are used for more complicated patterns can be added below.
    chest_blink: Option<ChestBlink>,
}

#[derive(Clone)]
struct ChestBlink {
    color: Color,
    interval: Duration,
    on: bool,
    start: Instant,
}

impl Led {
    pub fn chest_blink(&mut self, color: Color, interval: Duration) {
        self.chest_blink = Some(ChestBlink {
            color,
            interval,
            on: true,
            start: Instant::now(),
        });
    }
}

#[system]
fn write_led_values(led: &mut Led, ctrl_msg: &mut NaoControlMessage) -> Result<()> {
    ctrl_msg.chest = led.chest;
    ctrl_msg.left_foot = led.left_foot;
    ctrl_msg.right_foot = led.right_foot;

    ctrl_msg.left_ear = led.left_ear.clone();
    ctrl_msg.right_ear = led.right_ear.clone();
    ctrl_msg.left_eye = led.left_eye.clone();
    ctrl_msg.right_eye = led.right_eye.clone();
    ctrl_msg.skull = led.skull.clone();

    if let Some(mut blink) = led.chest_blink.clone() {
        if blink.start.elapsed() > blink.interval {
            blink.on = !blink.on;
            blink.start = Instant::now();
        }

        if blink.on {
            ctrl_msg.chest = blink.color;
        } else {
            ctrl_msg.chest = Color::GRAY;
        }
    }

    Ok(())
}
