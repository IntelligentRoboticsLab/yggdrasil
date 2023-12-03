use std::net::SocketAddr;

use bifrost::serialization::{Decode, Encode};

#[cfg(feature = "lola")]
use super::stream::{WebSocketReceiver, WebSocketSender};

#[cfg(feature = "lola")]
pub enum Message {
    Payload {
        address: SocketAddr,
        payload: DebugPayload,
    },
    OpenConnection {
        address: SocketAddr,
        tx: WebSocketSender,
        rx: WebSocketReceiver,
    },
    CloseConnection {
        address: SocketAddr,
    },
}

#[derive(Debug, Encode, Decode)]
pub enum DebugPayload {
    Text(String, String),
}
