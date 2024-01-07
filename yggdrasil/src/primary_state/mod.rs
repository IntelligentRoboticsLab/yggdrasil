use std::time::Duration;

use crate::{filter::button::HeadButtons, leds::Leds};
use bifrost::communication::{GameControllerMessage, GameState};
use miette::Result;
use nidhogg::types::Color;
use tyr::prelude::*;

const CHEST_BLINK_INTERVAL: Duration = Duration::from_millis(1000);

/// A module providing information about the primary state of the robot. These
/// states include: "Unstiff", "Initial", "Ready", "Set", "Playing",
/// "Penalized", "Finished" and "Calibration".
///
/// This module provides the following resources to the application:
/// - [`PrimaryState`]
pub struct PrimaryStateModule;

impl Module for PrimaryStateModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_resource(Resource::new(PrimaryState::Initial))?
            .add_system(update_primary_state))
    }
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub enum PrimaryState {
    /// State in which all joints are unstiffened and the robot does not move
    Unstiff,
    /// State at the start of the match where the robots stand up
    Initial,
    /// State in which robots walk to their legal positions
    Ready,
    /// State in which the robots wait for a kick-off or penalty
    Set,
    /// State in which the robots are playing soccer
    Playing,
    /// State when the robot has been penalized. Robot may not move except for
    /// standing up
    Penalized,
    /// State of the robot when a half is finished
    Finished,
    /// State the indicates the robot is performing automatic callibration
    Calibration,
}

#[system]
fn update_primary_state(
    primary_state: &mut PrimaryState,
    game_controller_message: &Option<GameControllerMessage>,
    led: &mut Leds,
    head_buttons: &HeadButtons,
) -> Result<()> {
    use PrimaryState as PS;

    // TODO: add penalized state
    // We need the robot's id and check the `RobotInfo` array in the game-controller message, to
    // see if this robot has received a penalty.

    let next_primary_state = match game_controller_message {
        Some(message) => match message {
            GameControllerMessage {
                state: GameState::Initial,
                ..
            } => PrimaryState::Initial,
            GameControllerMessage {
                state: GameState::Ready,
                ..
            } => PrimaryState::Ready,
            GameControllerMessage {
                state: GameState::Set,
                ..
            } => PrimaryState::Set,
            GameControllerMessage {
                state: GameState::Playing,
                ..
            } => PrimaryState::Playing,
            GameControllerMessage {
                state: GameState::Finished,
                ..
            } => PrimaryState::Finished,
        },
        None if head_buttons.middle.is_pressed() => PrimaryState::Unstiff,
        None => *primary_state,
    };

    // Only set color if the primary state is changed, with the exception of `Initial`.
    if next_primary_state != *primary_state {
        led.unset_chest_blink();

        match next_primary_state {
            PS::Unstiff => led.set_chest_blink(Color::BLUE, CHEST_BLINK_INTERVAL),
            PS::Initial => led.chest = Color::GRAY,
            PS::Ready => led.chest = Color::BLUE,
            PS::Set => led.chest = Color::YELLOW,
            PS::Playing => led.chest = Color::GREEN,
            PS::Penalized => led.chest = Color::RED,
            PS::Finished => led.chest = Color::GRAY,
            PS::Calibration => led.chest = Color::PURPLE,
        };
    } else if next_primary_state == PS::Initial {
        led.chest = Color::GRAY;
    }

    *primary_state = next_primary_state;

    Ok(())
}
