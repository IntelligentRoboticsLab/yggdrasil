use std::net::SocketAddr;

use bevy::prelude::*;
use bifrost::{communication::GameControllerMessage, serialization::Decode};
use futures::channel::mpsc::{self, UnboundedSender};

use super::{GameControllerConfig, GameControllerConnection, GameControllerSocket};

/// A new incoming [`GameControllerMessage`].
///
/// This event is emitted when a new [`GameControllerMessage`] is received.
#[derive(Deref, Event)]
pub struct GameControllerMessageEvent(pub GameControllerMessage);

#[derive(Resource)]
pub struct GameControllerReceiver {
    pub rx: mpsc::UnboundedReceiver<(GameControllerMessage, SocketAddr)>,
}

impl GameControllerReceiver {
    fn try_recv(&mut self) -> Option<(GameControllerMessage, SocketAddr)> {
        self.rx
            .try_next()
            .transpose()
            .expect("GameControllerMessage channel closed")
            .ok()
    }
}

pub async fn receive_loop(
    game_controller_socket: GameControllerSocket,
    tx: UnboundedSender<(GameControllerMessage, SocketAddr)>,
) {
    // The buffer is larger than necessary, in case we somehow receive invalid data, which can be a
    // bit longer than a normal `GameControllerMessage`.
    let mut buffer = [0u8; 2 * size_of::<GameControllerMessage>()];

    loop {
        let Ok((_size, address)) = game_controller_socket.recv_from(&mut buffer).await else {
            tracing::error!("Received invalid data from GameControllerSocket");
            continue;
        };

        if game_controller_socket
            .configured_game_controller_address()
            .is_some_and(|configured_game_controller_socket| {
                configured_game_controller_socket != address.ip()
            })
        {
            continue;
        }

        let Ok(message) = GameControllerMessage::decode(&mut buffer.as_slice()) else {
            tracing::error!("Could not decode GameControllerMessage");
            continue;
        };

        if message.is_valid() {
            tx.unbounded_send((message, address)).unwrap();
        }
    }
}

pub fn handle_messages(
    mut commands: Commands,
    mut receiver: ResMut<GameControllerReceiver>,
    mut connection: Option<ResMut<GameControllerConnection>>,
    time: Res<Time>,
    mut ev_message: EventWriter<GameControllerMessageEvent>,
    cfg: Res<GameControllerConfig>,
) {
    if let Some(conn) = &mut connection {
        // Tick the connection timeout
        conn.tick(time.delta());

        // And remove if it timed out
        if conn.timed_out() {
            tracing::info!("Lost gamecontroller connection with {}", conn.address);
            commands.remove_resource::<GameControllerConnection>();
        }
    }

    while let Some((message, address)) = receiver.try_recv() {
        assert!(
            message.is_valid(),
            "Handled GameControllerMessage should always be valid"
        );

        match connection.as_mut() {
            // If we already have a connection, reset the timeout
            Some(con) if con.address == address => {
                con.reset_timeout();
                ev_message.send(GameControllerMessageEvent(message));
            }
            // If we have a connection, but the message is from a different address, ignore
            Some(con) => {
                tracing::debug!(
                    "Received GameControllerMessage from unexpected address: {}",
                    con.address
                );
            }
            // If we don't have a connection, create one
            None => {
                commands.insert_resource(GameControllerConnection::new(
                    address,
                    cfg.game_controller_timeout,
                ));
                tracing::info!("Established gamecontroller connection with {}", address);
                ev_message.send(GameControllerMessageEvent(message));
            }
        }
    }
}
