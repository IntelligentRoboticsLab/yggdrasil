use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Instant;

use tokio::net::UdpSocket;

use bifrost::communication::GAMECONTROLLER_DATA_PORT;

use miette::{IntoDiagnostic, Result};
use tyr::prelude::*;

mod receive;
mod transmit;

pub(crate) struct GameControllerData {
    pub socket: Arc<UdpSocket>,
    pub last_send_message_instant: Instant,
    pub game_controller_address: Option<SocketAddr>,
}

pub struct GameControllerModule;

impl GameControllerModule {
    async fn new_game_controller_socket() -> Result<UdpSocket> {
        let game_controller_socket = UdpSocket::bind(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            GAMECONTROLLER_DATA_PORT,
        ))
        .await
        .into_diagnostic()?;

        Ok(game_controller_socket)
    }

    fn init_udp_socket(storage: &mut Storage) -> Result<()> {
        let game_controller_socket =
            storage.map_resource_ref(|async_dispatcher: &AsyncDispatcher| {
                async_dispatcher
                    .handle()
                    .block_on(Self::new_game_controller_socket())
            })??;

        storage.add_resource(Resource::new(GameControllerData {
            last_send_message_instant: Instant::now(),
            socket: Arc::new(game_controller_socket),
            game_controller_address: None,
        }))?;

        Ok(())
    }
}

impl Module for GameControllerModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_startup_system(Self::init_udp_socket)?
            .add_module(receive::GameControllerReceiveModule)?
            .add_module(transmit::GameControllerSendModule)
    }
}
