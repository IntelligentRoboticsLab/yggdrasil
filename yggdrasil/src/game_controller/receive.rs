use super::GameControllerData;

use bifrost::communication::RoboCupGameControlData;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::UdpSocket;

use bifrost::serialization::Decode;

use miette::{IntoDiagnostic, Result};

use tyr::prelude::*;

pub struct GameControllerReceiveModule;

impl Module for GameControllerReceiveModule {
    fn initialize(self, app: App) -> Result<App> {
        let game_controller_receive_message = Resource::<Option<RoboCupGameControlData>>::new(None);
        let game_controller_address = Resource::<Option<SocketAddr>>::new(None);

        Ok(app
            .add_task::<AsyncTask<Result<(RoboCupGameControlData, SocketAddr)>>>()?
            .add_resource(game_controller_receive_message)?
            .add_resource(game_controller_address)?
            .add_startup_system(init_receive_game_controller_data_task)?
            .add_system(receive_system))
    }
}

fn init_receive_game_controller_data_task(storage: &mut Storage) -> Result<()> {
    let game_controller_socket =
        storage.map_resource_mut(|game_controller_data: &mut GameControllerData| {
            game_controller_data.socket.clone()
        })?;

    storage.map_resource_mut(
        |receive_game_controller_data_task: &mut AsyncTask<
            Result<(RoboCupGameControlData, SocketAddr)>,
        >| {
            receive_game_controller_data_task
                .try_spawn(receive_game_controller_data(game_controller_socket))
        },
    )??;

    Ok(())
}

async fn receive_game_controller_data(
    game_controller_socket: Arc<UdpSocket>,
) -> Result<(RoboCupGameControlData, SocketAddr)> {
    let mut buffer = [0u8; 1024];

    let (_bytes_received, new_game_controller_address) = game_controller_socket
        .recv_from(&mut buffer)
        .await
        .into_diagnostic()?;

    let new_game_controller_message =
        RoboCupGameControlData::decode(&mut buffer.as_slice()).into_diagnostic()?;

    Ok((new_game_controller_message, new_game_controller_address))
}

#[system]
pub(crate) fn receive_system(
    game_controller_message: &mut Option<RoboCupGameControlData>,
    game_controller_data: &mut GameControllerData,
    receive_game_controller_data_task: &mut AsyncTask<Result<(RoboCupGameControlData, SocketAddr)>>,
) -> Result<()> {
    match receive_game_controller_data_task.poll() {
        Some(Ok((new_game_controller_message, new_game_controller_address))) => {
            *game_controller_message = Some(new_game_controller_message);
            game_controller_data.game_controller_address = Some(new_game_controller_address);

            receive_game_controller_data_task
                .try_spawn(receive_game_controller_data(
                    game_controller_data.socket.clone(),
                ))
                .into_diagnostic()?;
        }
        Some(Err(error)) => tracing::warn!("Failed to decode game controller message: {error}"),
        None => {}
    }

    Ok(())
}
