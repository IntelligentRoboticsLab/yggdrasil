use crate::{filter::button::ChestButton, leds::Leds, prelude::*};

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};
use std::time::Duration;

use bifrost::communication::{GameControllerMessage, GameState};
use nidhogg::types::Color;

#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct PrimaryStateConfig {
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub chest_blink_interval: Duration,
}

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
            .add_resource(Resource::new(PrimaryState::Unstiff))?
            .add_system(update_primary_state.after(crate::filter::button::button_filter)))
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

impl PrimaryState {
    /// Tell whether the robot should walk in this state.
    pub fn should_walk(&self) -> bool {
        !matches!(
            self,
            Self::Unstiff | Self::Penalized | Self::Finished | Self::Calibration
        )
    }
}

#[system]
pub fn update_primary_state(
    primary_state: &mut PrimaryState,
    game_controller_message: &Option<GameControllerMessage>,
    led: &mut Leds,
    chest_button: &ChestButton,
    config: &PrimaryStateConfig,
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
        None if chest_button.state.is_tapped() => PrimaryState::Initial,
        None => *primary_state,
    };

    // Only set color if the primary state is changed, with the exception of `Initial`.
    if next_primary_state != *primary_state {
        led.unset_chest_blink();

        match next_primary_state {
            PS::Unstiff => led.set_chest_blink(Color::BLUE, config.chest_blink_interval),
            PS::Initial => led.chest = Color::GRAY,
            PS::Ready => led.chest = Color::BLUE,
            PS::Set => led.chest = Color::YELLOW,
            PS::Playing => led.chest = Color::GREEN,
            PS::Penalized => led.chest = Color::RED,
            PS::Finished => led.chest = Color::GRAY,
            PS::Calibration => led.chest = Color::PURPLE,
        };
    } else if next_primary_state == PS::Unstiff {
        led.set_chest_blink(Color::BLUE, config.chest_blink_interval)
    }

    *primary_state = next_primary_state;

    Ok(())
}
