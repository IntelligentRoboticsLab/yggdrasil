mod receive;
mod transmit;

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use async_std::{
    io,
    net::{ToSocketAddrs, UdpSocket},
};
use bevy::{
    prelude::*,
    tasks::{block_on, IoTaskPool},
};
use bifrost::communication::{
    GameControllerMessage, GameControllerReturnMessage, GAME_CONTROLLER_DATA_PORT,
};
use futures::channel::mpsc;
use receive::{handle_messages, receive_loop, GameControllerReceiver};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds};
use transmit::{send_loop, send_message, GameControllerSender};

pub use receive::GameControllerMessageEvent;

/// This module handles the communication with the game controller.
///
/// The received game controller messages are emitted as [`GameControllerMessageEvent`] events.
///
/// If connection to the game controller has been lost for an extended period of time, the connection will
/// be forgotten and a new game controller is allowed to connect.
///
/// The module also transmits status updates back to the game-controller. These messages include data like
/// the robot's number and position.
///
/// This module provides the following events to the application:
/// - [`GameControllerMessageEvent`]
///
/// This module provides the following resources to the application:
/// - [`GameControllerConfig`]
///
pub struct GameControllerPlugin;

impl Plugin for GameControllerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<GameControllerMessageEvent>()
            .add_systems(Startup, setup)
            .add_systems(PreUpdate, handle_messages)
            .add_systems(
                PostUpdate,
                send_message.run_if(resource_exists::<GameControllerConnection>),
            );
    }
}

/// Configuration for the game controller.
#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone, Resource)]
#[serde(deny_unknown_fields)]
pub struct GameControllerConfig {
    /// The timeout for the game controller connection.
    ///
    /// If no message is received from the game controller within this time, the connection is considered lost.
    ///
    /// Allows a new game controller to connect after an old one has disconnected.
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub game_controller_timeout: Duration,
    /// The delay between sending return messages to the game controller.
    ///
    /// Used to limit the rate at which the game controller is updated.
    #[serde_as(as = "DurationMilliSeconds<u64>")]
    pub game_controller_return_delay: Duration,
}

#[derive(Resource)]
struct GameControllerConnection {
    address: SocketAddr,
    timer: Timer,
}

impl GameControllerConnection {
    pub fn new(address: SocketAddr, timeout: Duration) -> Self {
        Self {
            address,
            timer: Timer::new(timeout, TimerMode::Once),
        }
    }

    pub fn tick(&mut self, delta: Duration) {
        self.timer.tick(delta);
    }

    pub fn reset_timeout(&mut self) {
        self.timer.reset();
    }

    pub fn timed_out(&self) -> bool {
        self.timer.finished()
    }
}

#[derive(Clone)]
struct GameControllerSocket {
    socket: Arc<UdpSocket>,
}

impl GameControllerSocket {
    async fn bind() -> io::Result<Self> {
        let socket =
            Arc::new(UdpSocket::bind((Ipv4Addr::UNSPECIFIED, GAME_CONTROLLER_DATA_PORT)).await?);

        Ok(Self { socket })
    }

    pub async fn recv_from(&self, buffer: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.socket.recv_from(buffer).await
    }

    pub async fn send_to<A: ToSocketAddrs>(&self, buffer: &[u8], addr: A) -> io::Result<usize> {
        self.socket.send_to(buffer, addr).await
    }
}

fn setup(mut commands: Commands) {
    let (tx_recv, rx_recv) = mpsc::unbounded::<(GameControllerMessage, SocketAddr)>();
    let (tx_send, rx_send) = mpsc::unbounded::<(GameControllerReturnMessage, SocketAddr)>();

    let io = IoTaskPool::get();
    let socket = block_on(io.spawn(GameControllerSocket::bind()))
        .expect("Failed to bind game controller socket");

    io.spawn(receive_loop(socket.clone(), tx_recv)).detach();
    io.spawn(send_loop(socket, rx_send)).detach();

    commands.insert_resource(GameControllerReceiver { rx: rx_recv });
    commands.insert_resource(GameControllerSender { tx: tx_send });
}
