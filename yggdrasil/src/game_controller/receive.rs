use super::GameControllerData;

use bifrost::communication::RoboCupGameControlData;
use bifrost::serialization::Decode;

use std::{
    io,
    mem::size_of,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::net::UdpSocket;

use miette::{IntoDiagnostic, Result};
use tyr::prelude::*;

const GAME_CONTROLLER_TIMEOUT_MS: u64 = 5000;

pub struct GameControllerReceiveModule;

impl Module for GameControllerReceiveModule {
    fn initialize(self, app: App) -> Result<App> {
        let game_controller_receive_message = Resource::<Option<RoboCupGameControlData>>::new(None);

        Ok(app
            .add_task::<AsyncTask<Result<(RoboCupGameControlData, SocketAddr)>>>()?
            .add_resource(game_controller_receive_message)?
            .add_startup_system(init_receive_game_controller_data_task)?
            .add_system(receive_system)
            .add_system(check_game_controller_connection_system.after(receive_system)))
    }
}

fn init_receive_game_controller_data_task(storage: &mut Storage) -> Result<()> {
    let game_controller_socket =
        storage.map_resource_mut(|game_controller_data: &mut GameControllerData| {
            game_controller_data.socket.clone()
        })?;

    storage
        .map_resource_mut(
            |receive_game_controller_data_task: &mut AsyncTask<
                Result<(RoboCupGameControlData, SocketAddr)>,
            >| {
                receive_game_controller_data_task
                    .try_spawn(receive_game_controller_data(game_controller_socket))
            },
        )?
        .into_diagnostic()
}

async fn receive_game_controller_data(
    game_controller_socket: Arc<UdpSocket>,
) -> Result<(RoboCupGameControlData, SocketAddr)> {
    // The buffer is larger than necesary, in case we somehow receive invalid data, which can be a
    // bit longer than a normal `RoboCupGameControlData`.
    let mut buffer = [0u8; 2 * size_of::<RoboCupGameControlData>()];

    let (_bytes_received, new_game_controller_address) = game_controller_socket
        .recv_from(&mut buffer)
        .await
        .into_diagnostic()?;

    let new_game_controller_message =
        RoboCupGameControlData::decode(&mut buffer.as_slice()).into_diagnostic()?;

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
fn contains_our_team_number(game_controller_message: &RoboCupGameControlData) -> bool {
    // TODO: Replace with value from Odal.
    let team_number = 8;

    let teams = game_controller_message.teams;
    teams[0].team_number == team_number || teams[1].team_number == team_number
}

#[system]
pub(super) fn receive_system(
    game_controller_message: &mut Option<RoboCupGameControlData>,
    game_controller_data: &mut GameControllerData,
    receive_game_controller_data_task: &mut AsyncTask<Result<(RoboCupGameControlData, SocketAddr)>>,
) -> Result<()> {
    let Some(receive_task_result) = receive_game_controller_data_task.poll() else {
        return Ok(());
    };

    match receive_task_result {
        Ok((new_game_controller_message, new_game_controller_address)) => {
            // If the new game-controller message doesn't contain our team number, we ignore it.
            // If the new game-controller message came from a different game controller than the
            // last message we received less than `GAME_CONTROLLER_TIMEOUT_MS`, we ignore
            // it as well, because it means that there are multiple game controllers active on
            // the network.
            if contains_our_team_number(&new_game_controller_message)
                && !game_controller_data
                    .game_controller_address
                    .is_some_and(|(old_address, _)| old_address != new_game_controller_address)
            {
                *game_controller_message = Some(new_game_controller_message);
                game_controller_data.game_controller_address =
                    Some((new_game_controller_address, Instant::now()));
            }
        }
        Err(error) => tracing::warn!("Failed to decode game controller message: {error}"),
    }

    receive_game_controller_data_task
        .try_spawn(receive_game_controller_data(
            game_controller_data.socket.clone(),
        ))
        .into_diagnostic()?;

    Ok(())
}

#[system]
fn check_game_controller_connection_system(
    game_controller_message: &mut Option<RoboCupGameControlData>,
    game_controller_data: &mut GameControllerData,
) -> Result<()> {
    if game_controller_data
        .game_controller_address
        .is_some_and(|(_, instant)| {
            instant.elapsed() > Duration::from_millis(GAME_CONTROLLER_TIMEOUT_MS)
        })
    {
        tracing::warn!("Lost connection to the game controller");

        *game_controller_message = None;
        game_controller_data.game_controller_address = None;
    }

    Ok(())
}
