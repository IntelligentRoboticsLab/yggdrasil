use bifrost::communication::GAME_CONTROLLER_DATA_PORT;

use tokio::net::UdpSocket;

use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::{Duration, Instant};

use miette::IntoDiagnostic;

use crate::prelude::*;

mod receive;
mod transmit;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct GameControllerConfig {
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub game_controller_timeout: Duration,
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub game_controller_return_delay: Duration,
}

struct GameControllerData {
    pub socket: Arc<UdpSocket>,
    pub game_controller_address: Option<(SocketAddr, Instant)>,
}

/// This module handles the communication with the game-controller.
///
/// The last received game-controller message is stored in a resource. If no message has been
/// received yet from the game-controller, or if connection to the game-controller has been lost
/// for an extended period of time, that resource is set to `None`.
///
/// The module transmits status updates back to the game-controller. These messages include data like
/// the robot's- number and position.
///
/// This module provides the following resources to the application:
/// - <code>[Option]<[GameControllerMessage](bifrost::communication::GameControllerMessage)></code>
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

    #[startup_system]
    fn add_resources(storage: &mut Storage, dispatcher: &AsyncDispatcher) -> Result<()> {
        let game_controller_socket = dispatcher
            .handle()
            .block_on(Self::new_game_controller_socket())?;

        storage.add_resource(Resource::new(GameControllerData {
            socket: Arc::new(game_controller_socket),
            game_controller_address: None,
        }))?;

        Ok(())
    }
}

impl Module for GameControllerModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_startup_system(Self::add_resources)?
            .add_module(receive::GameControllerReceiveModule)?
            .add_module(transmit::GameControllerTransmitModule)
    }
}
