use super::GameControllerSocket;

use bifrost::communication::RoboCupGameControlData;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use bifrost::serialization::Decode;

use miette::{IntoDiagnostic, Result};

use tyr::prelude::*;

pub struct GameControllerReceiveModule;

impl Module for GameControllerReceiveModule {
    fn initialize(self, app: App) -> Result<App> {
        let game_controller_receive_message = Resource::<Option<RoboCupGameControlData>>::new(None);
        let game_controller_address = Resource::<Option<SocketAddr>>::new(None);

        Ok(app
            .add_resource(game_controller_receive_message)?
            .add_resource(game_controller_address)?
            .add_task::<AsyncTask<Result<(RoboCupGameControlData, SocketAddr)>>>()?
            .add_system(receive_system))
    }
}

async fn receive_message(
    game_controller_socket: Arc<Mutex<GameControllerSocket>>,
) -> Result<(RoboCupGameControlData, SocketAddr)> {
    let mut buffer = vec![0u8; 1024];

    let (_bytes_received, game_controller_address) = game_controller_socket
        .lock()
        .unwrap()
        .recv_from(&mut buffer)
        .into_diagnostic()?;

    let message = RoboCupGameControlData::decode(&mut buffer.as_slice()).into_diagnostic()?;

    Ok((message, game_controller_address))
}

#[system]
pub(crate) fn receive_system(
    game_controller_message: &mut Option<RoboCupGameControlData>,
    game_controller_socket: &mut Arc<Mutex<GameControllerSocket>>,
    game_controller_address: &mut Option<SocketAddr>,
    receive_message_task: &mut AsyncTask<Result<(RoboCupGameControlData, SocketAddr)>>,
) -> Result<()> {
    if !receive_message_task.active() {
        receive_message_task
            .try_spawn(receive_message(game_controller_socket.clone()))
            .into_diagnostic()?;
    } else if let Some(Ok((new_game_controller_message, new_game_controller_address))) =
        receive_message_task.poll()
    {
        *game_controller_message = Some(new_game_controller_message);
        *game_controller_address = Some(new_game_controller_address);
    }

    Ok(())
}
