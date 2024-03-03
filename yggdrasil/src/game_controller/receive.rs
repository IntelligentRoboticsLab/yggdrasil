use super::GameControllerData;

use super::GameControllerConfig;
use bifrost::communication::GameControllerMessage;

use bifrost::serialization::Decode;
use miette::IntoDiagnostic;

use std::{io, mem::size_of, net::SocketAddr, sync::Arc, time::Instant};

use tokio::net::UdpSocket;

use crate::prelude::*;

pub(super) struct GameControllerReceiveModule;

impl Module for GameControllerReceiveModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_task::<AsyncTask<Result<(GameControllerMessage, SocketAddr)>>>()?
            .init_resource::<Option<GameControllerMessage>>()?
            .add_startup_system(init_receive_game_controller_message_task)?
            .add_system(receive_system)
            .add_system(check_game_controller_connection_system.after(receive_system)))
    }
}

#[startup_system]
fn init_receive_game_controller_message_task(
    _storage: &mut Storage,
    game_controller_data: &mut GameControllerData,
    receive_game_controller_message_task: &mut AsyncTask<
        Result<(GameControllerMessage, SocketAddr)>,
    >,
) -> Result<()> {
    let game_controller_socket = game_controller_data.socket.clone();

    receive_game_controller_message_task
        .try_spawn(receive_game_controller_message(game_controller_socket))
        .into_diagnostic()
}

async fn receive_game_controller_message(
    game_controller_socket: Arc<UdpSocket>,
) -> Result<(GameControllerMessage, SocketAddr)> {
    // The buffer is larger than necessary, in case we somehow receive invalid data, which can be a
    // bit longer than a normal `GameControllerMessage`.
    let mut buffer = [0u8; 2 * size_of::<GameControllerMessage>()];

    let (_bytes_received, new_game_controller_address) = game_controller_socket
        .recv_from(&mut buffer)
        .await
        .into_diagnostic()?;

    let new_game_controller_message =
        GameControllerMessage::decode(&mut buffer.as_slice()).into_diagnostic()?;

    if !new_game_controller_message.is_valid() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Received invalid data from the game controller",
        ))
        .into_diagnostic();
    }

    Ok((new_game_controller_message, new_game_controller_address))
}

/// Check if the game controller message contains our team-number.
fn contains_our_team_number(game_controller_message: &GameControllerMessage) -> bool {
    // TODO: Replace with value from Odal.
    let team_number = 8;

    let teams = game_controller_message.teams;
    teams[0].team_number == team_number || teams[1].team_number == team_number
}

// Check if we should replace the game-controller message.
//
// First, check whether the new game-controller message came from the currently connected game-controller.
// If it does, we should replace the game-controller message, because this newer message is simply
// an update. If the newer game-controller message came from a different game-controller, we should
// ignore it.
//
// If we're not connected to a game-controller at all (`old_game_controller_address` ==
// None), then we should also replace the game-controller message.
fn should_replace_old_game_controller_message(
    old_game_controller_address: Option<(SocketAddr, Instant)>,
    new_game_controller_address: SocketAddr,
) -> bool {
    old_game_controller_address.map_or(true, |(old_game_controller_address, _)| {
        old_game_controller_address.ip() == new_game_controller_address.ip()
    })
}

#[system]
fn receive_system(
    game_controller_message: &mut Option<GameControllerMessage>,
    game_controller_data: &mut GameControllerData,
    receive_game_controller_message_task: &mut AsyncTask<
        Result<(GameControllerMessage, SocketAddr)>,
    >,
) -> Result<()> {
    let Some(receive_task_result) = receive_game_controller_message_task.poll() else {
        return Ok(());
    };

    match receive_task_result {
        Ok((new_game_controller_message, new_game_controller_address)) => {
            if contains_our_team_number(&new_game_controller_message)
                && should_replace_old_game_controller_message(
                    game_controller_data.game_controller_address,
                    new_game_controller_address,
                )
            {
                *game_controller_message = Some(new_game_controller_message);
                game_controller_data.game_controller_address =
                    Some((new_game_controller_address, Instant::now()));
            }
        }
        Err(error) => {
            tracing::warn!("Failed to decode game controller message: {error}");
        }
    };

    receive_game_controller_message_task
        .try_spawn(receive_game_controller_message(
            game_controller_data.socket.clone(),
        ))
        .into_diagnostic()?;

    Ok(())
}

#[system]
fn check_game_controller_connection_system(
    game_controller_message: &mut Option<GameControllerMessage>,
    game_controller_data: &mut GameControllerData,
    config: &GameControllerConfig,
) -> Result<()> {
    if game_controller_data
        .game_controller_address
        .is_some_and(|(_, timestamp)| timestamp.elapsed() > config.game_controller_timeout)
    {
        tracing::warn!("Lost connection to the game controller");

        *game_controller_message = None;
        game_controller_data.game_controller_address = None;
    }

    Ok(())
}
