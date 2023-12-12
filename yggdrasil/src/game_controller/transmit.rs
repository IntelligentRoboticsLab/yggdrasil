use super::GameControllerData;

use bifrost::communication::{RoboCupGameControlReturnData, GAMECONTROLLER_RETURN_PORT};
use bifrost::serialization::Encode;

use std::ops::Add;
use std::time::{Duration, Instant};

use miette::{IntoDiagnostic, Result};
use tyr::prelude::*;

const GAMECONTROLLER_RETURN_DELAY_MS: u64 = 500;

pub struct GameControllerSendModule;

impl Module for GameControllerSendModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app.add_system(send_system))
    }
}

#[system]
pub(crate) fn send_system(game_controller_data: &mut GameControllerData) -> Result<()> {
    let Some(mut game_controller_address) = game_controller_data.game_controller_address else {
        return Ok(());
    };

    let duration_to_wait = game_controller_data
        .last_send_message_instant
        .add(Duration::from_millis(GAMECONTROLLER_RETURN_DELAY_MS))
        .duration_since(Instant::now());

    if !duration_to_wait.is_zero() {
        return Ok(());
    }

    let mut buffer = [0u8; 1024];

    game_controller_address.set_port(GAMECONTROLLER_RETURN_PORT);
    let game_controller_message =
        RoboCupGameControlReturnData::new(2, 8, 0, [0f32; 3], -1f32, [0f32; 2]);

    game_controller_message
        .encode(buffer.as_mut_slice())
        .into_diagnostic()?;

    game_controller_data
        .socket
        .send_to(buffer.as_slice(), game_controller_address)
        .into_diagnostic()?;

    game_controller_data.last_send_message_instant = Instant::now();

    Ok(())
}
