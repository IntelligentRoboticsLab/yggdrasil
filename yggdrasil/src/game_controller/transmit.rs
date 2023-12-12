use super::GameControllerSocket;

use bifrost::communication::{RoboCupGameControlReturnData, GAMECONTROLLER_RETURN_PORT};
use bifrost::serialization::Encode;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use miette::{IntoDiagnostic, Result};
use tyr::prelude::*;

pub struct GameControllerSendModule;

impl Module for GameControllerSendModule {
    fn initialize(self, app: App) -> Result<App> {
        let game_controller_return_message = Resource::<Option<()>>::new(None);

        Ok(app
            .add_resource(game_controller_return_message)?
            .add_task::<AsyncTask<Result<RoboCupGameControlReturnData>>>()?
            .add_system(send_system))
    }
}

async fn send_message(
    game_controller_socket: Arc<Mutex<GameControllerSocket>>,
    mut game_controller_address: SocketAddr,
) -> Result<RoboCupGameControlReturnData> {
    let mut buffer = [0u8; 1024];

    game_controller_address.set_port(GAMECONTROLLER_RETURN_PORT);
    let game_controller_message =
        RoboCupGameControlReturnData::new(2, 8, 0, [0f32; 3], -1f32, [0f32; 2]);

    std::thread::sleep(Duration::from_secs(1));

    game_controller_message
        .encode(buffer.as_mut_slice())
        .into_diagnostic()?;

    game_controller_socket
        .lock()
        .unwrap()
        .send_to(buffer.as_slice(), game_controller_address)
        .into_diagnostic()?;

    Ok(game_controller_message)
}

#[system]
pub fn send_system(
    game_controller_socket: &mut Arc<Mutex<GameControllerSocket>>,
    game_controller_return_address: &Option<SocketAddr>,
    send_message_task: &mut AsyncTask<Result<RoboCupGameControlReturnData>>,
) -> Result<()> {
    if let Some(game_controller_return_address) = game_controller_return_address {
        if !send_message_task.active() {
            send_message_task
                .try_spawn(send_message(
                    game_controller_socket.clone(),
                    *game_controller_return_address,
                ))
                .into_diagnostic()?;
        } else if let Some(_new_game_controller_message) = send_message_task.poll() {
        }
    }

    Ok(())
}
