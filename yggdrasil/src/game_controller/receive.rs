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
            .add_system(receive_system))
    }
}

#[system]
pub(crate) fn receive_system(
    game_controller_message: &mut Option<RoboCupGameControlData>,
    game_controller_socket: &mut Arc<Mutex<GameControllerSocket>>,
    game_controller_address: &mut Option<SocketAddr>,
) -> Result<()> {
    let mut buffer = [0u8; 1024];

    match game_controller_socket
        .lock()
        .unwrap()
        .recv_from(&mut buffer)
    {
        Ok((_bytes_received, new_game_controller_address)) => {
            let new_game_controller_message =
                RoboCupGameControlData::decode(&mut buffer.as_slice()).into_diagnostic()?;

            *game_controller_message = Some(new_game_controller_message);
            *game_controller_address = Some(new_game_controller_address);

            Ok(())
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(()),
        Err(err) => Err(err).into_diagnostic()?,
    }
}
