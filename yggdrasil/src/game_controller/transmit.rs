use bifrost::communication::RoboCupGameControlReturnData;
use bifrost::serialization::Encode;

use std::net::SocketAddr;
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};

use miette::{IntoDiagnostic, Result};
use tyr::prelude::*;

pub struct GameControllerSendModule;

async fn send_message(
    game_controller_socket: Arc<Mutex<UdpSocket>>,
    game_controller_return_address: SocketAddr,
) -> Result<RoboCupGameControlReturnData> {
    let mut buffer = vec![0u8; 1024];

    let game_controller_return_message =
        RoboCupGameControlReturnData::new(1, 8, 0, [0f32, 0f32, 0f32], -1f32, [0f32, 0f32]);

    eprintln!("{:?}", game_controller_return_address);

    game_controller_return_message
        .encode(&mut buffer)
        .into_diagnostic()?;
    game_controller_socket
        .lock()
        .unwrap()
        .send_to(buffer.as_slice(), game_controller_return_address)
        .into_diagnostic()?;

    Ok(game_controller_return_message)
}

impl Module for GameControllerSendModule {
    fn initialize(self, app: App) -> Result<App> {
        let game_controller_return_message = Resource::<Option<()>>::new(None);

        Ok(app
            .add_resource(game_controller_return_message)?
            .add_task::<AsyncTask<Result<RoboCupGameControlReturnData>>>()?
            .add_system(send_system))
    }
}

#[system]
pub fn send_system(
    game_controller_socket: &mut Arc<Mutex<UdpSocket>>,
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
