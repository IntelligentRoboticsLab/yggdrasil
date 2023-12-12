use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use bifrost::communication::GAMECONTROLLER_DATA_PORT;

use miette::{IntoDiagnostic, Result};
use tyr::prelude::*;

mod receive;
mod transmit;

pub struct GameControllerModule;

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

impl Module for GameControllerModule {
    fn initialize(self, app: App) -> Result<App> {
        let game_controller_socket = GameControllerSocket(
            UdpSocket::bind(SocketAddrV4::new(
                Ipv4Addr::UNSPECIFIED,
                GAMECONTROLLER_DATA_PORT,
            ))
            .into_diagnostic()?,
        );

        game_controller_socket
            .set_nonblocking(true)
            .into_diagnostic()?;

        let game_controller_socket_resource =
            Resource::new(Arc::new(Mutex::new(game_controller_socket)));

        Ok(app
            .add_resource(game_controller_socket_resource)?
            .add_module(receive::GameControllerReceiveModule)?
            .add_module(transmit::GameControllerSendModule)?)
    }
}
