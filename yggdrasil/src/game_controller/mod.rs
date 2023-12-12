use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::Instant;

use bifrost::communication::GAMECONTROLLER_DATA_PORT;

use miette::{IntoDiagnostic, Result};
use tyr::prelude::*;

mod receive;
mod transmit;

pub(crate) struct GameControllerData {
    pub socket: UdpSocket,
    pub last_send_message_instant: Instant,
    pub game_controller_address: Option<SocketAddr>,
}

pub struct GameControllerModule;

impl GameControllerModule {
    fn new_game_controller_socket() -> Result<UdpSocket> {
        let game_controller_socket = UdpSocket::bind(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            GAMECONTROLLER_DATA_PORT,
        ))
        .into_diagnostic()?;

        game_controller_socket
            .set_nonblocking(true)
            .into_diagnostic()?;

        Ok(game_controller_socket)
    }

    fn new_game_controller_data() -> Result<GameControllerData> {
        let game_controller_socket = Self::new_game_controller_socket()?;

        Ok(GameControllerData {
            last_send_message_instant: Instant::now(),
            socket: game_controller_socket,
            game_controller_address: None,
        })
    }
}

impl Module for GameControllerModule {
    fn initialize(self, app: App) -> Result<App> {
        let game_controller_data = Self::new_game_controller_data()?;

        let game_controller_data_resource =
            Resource::<GameControllerData>::new(game_controller_data);

        app.add_resource(game_controller_data_resource)?
            .add_module(receive::GameControllerReceiveModule)?
            .add_module(transmit::GameControllerSendModule)
    }
}
