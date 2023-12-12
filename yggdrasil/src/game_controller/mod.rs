use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};
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

pub struct GameControllerSocket(UdpSocket);

impl Deref for GameControllerSocket {
    type Target = UdpSocket;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for GameControllerSocket {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct GameControllerModule;

impl Module for GameControllerModule {
    fn initialize(self, app: App) -> Result<App> {
        let game_controller_socket = UdpSocket::bind(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            GAMECONTROLLER_DATA_PORT,
        ))
        .into_diagnostic()?;

        game_controller_socket
            .set_nonblocking(true)
            .into_diagnostic()?;

        let game_controller_data = GameControllerData {
            last_send_message_instant: Instant::now(),
            socket: game_controller_socket,
            game_controller_address: None,
        };
        let game_controller_data_resource = Resource::<Arc<Mutex<GameControllerData>>>::new(
            Arc::new(Mutex::new(game_controller_data)),
        );

        Ok(app
            .add_resource(game_controller_data_resource)?
            .add_module(receive::GameControllerReceiveModule)?
            .add_module(transmit::GameControllerSendModule)?)
    }
}
