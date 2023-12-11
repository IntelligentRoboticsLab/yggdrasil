use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
use std::sync::{Arc, Mutex};

use bifrost::communication::GAMECONTROLLER_DATA_PORT;

use miette::{IntoDiagnostic, Result};
use tyr::prelude::*;

mod receive;
mod transmit;

pub struct GameControllerModule;

impl Module for GameControllerModule {
    fn initialize(self, app: App) -> Result<App> {
        let game_controller_socket = Resource::new(Arc::new(Mutex::new(
            UdpSocket::bind(SocketAddrV4::new(
                Ipv4Addr::UNSPECIFIED,
                GAMECONTROLLER_DATA_PORT,
            ))
            .into_diagnostic()?,
        )));

        Ok(app
            .add_resource(game_controller_socket)?
            .add_module(receive::GameControllerReceiveModule)?
            .add_module(transmit::GameControllerSendModule)?)
    }
}
