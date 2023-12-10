use std::time::Duration;

use crate::leds::Led;
use miette::Result;
use nidhogg::types::Color;
use tyr::prelude::*;

pub struct PrimaryStateModule;

impl Module for PrimaryStateModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_resource(Resource::new(PrimaryState::Initial))?
            .add_system(update_primary_state)
            .add_system(show_primary_state.after(update_primary_state)))
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum PrimaryState {
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
#[allow(unused_variables)]
fn update_primary_state(primary_state: &mut PrimaryState) -> Result<()> {
    // TODO: update primary state based on gamecontroller messages.
    Ok(())
}

#[system]
fn show_primary_state(primary_state: &mut PrimaryState, led: &mut Led) -> Result<()> {
    use PrimaryState as PS;

    match *primary_state {
        PS::Unstiff => led.chest_blink(Color::BLUE, Duration::from_secs(1)),
        PS::Initial => led.chest = Color::GRAY,
        PS::Ready => led.chest = Color::BLUE,
        PS::Set => led.chest = Color::YELLOW,
        PS::Playing => led.chest = Color::GREEN,
        PS::Penalized => led.chest = Color::RED,
        PS::Finished => led.chest = Color::GRAY,
        PS::Calibration => led.chest = Color::PURPLE,
    };

    Ok(())
}
