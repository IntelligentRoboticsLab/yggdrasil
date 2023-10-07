use std::time::Duration;

use crate::leds::Led;
use miette::Result;
use nidhogg::types::Color;
use tyr::prelude::*;

pub struct RobotPrimaryStateModule;

impl Module for RobotPrimaryStateModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_resource(Resource::new(RobotPrimaryState::Initial))?
            .add_system(show_primary_state)
            .add_system(update_primary_state))
    }
}

#[allow(dead_code)]
pub enum RobotPrimaryState {
    Unstiff,
    Initial,
    Ready,
    Set,
    Playing,
    Penalized,
    Finished,
    Calibration,
}

#[system]
fn update_primary_state(_primary_state: &mut RobotPrimaryState) -> Result<()> {
    // TODO: update primary state based on gamecontroller messages.
    Ok(())
}

#[system]
fn show_primary_state(primary_state: &mut RobotPrimaryState, led: &mut Led) -> Result<()> {
    use RobotPrimaryState::*;

    match *primary_state {
        Unstiff => led.chest_blink(Color::BLUE, Duration::from_secs(1)),
        Initial => led.chest = Color::GRAY,
        Ready => led.chest = Color::BLUE,
        Set => led.chest = Color::YELLOW,
        Playing => led.chest = Color::GREEN,
        Penalized => led.chest = Color::RED,
        Finished => led.chest = Color::GRAY,
        Calibration => led.chest = Color::PURPLE,
    };

    Ok(())
}
