use bifrost::communication::GAME_CONTROLLER_DATA_PORT;

use tokio::net::UdpSocket;

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Instant;

use miette::{IntoDiagnostic, Result};
use tyr::prelude::*;

mod receive;
mod transmit;

pub(crate) struct GameControllerConfig {
    pub socket: Arc<UdpSocket>,
    pub last_send_message_timestamp: Instant,
    pub game_controller_address: Option<(SocketAddr, Instant)>,
}

/// This module handles the communication with the game-controller.
///
/// The last received game-controller message is stored in an resource. If no message has been
/// received yet from the game-controller, or if connection to the game-controller has been lost
/// for an extended period of time, that resource is set to `None`.
///
/// The module send status updates back to the game-controller. These messages include data like
/// the robot's- number and position.
///
/// This module provides the following resources to the application:
/// - [`Option`]<[`GameControllerData`](bifrost::communication::GameControllerData)>
pub struct GameControllerModule;

impl GameControllerModule {
    async fn new_game_controller_socket() -> Result<UdpSocket> {
        let game_controller_socket = UdpSocket::bind(SocketAddrV4::new(
            Ipv4Addr::UNSPECIFIED,
            GAME_CONTROLLER_DATA_PORT,
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

        storage.add_resource(Resource::new(GameControllerConfig {
            socket: Arc::new(game_controller_socket),
            last_send_message_timestamp: Instant::now(),
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
