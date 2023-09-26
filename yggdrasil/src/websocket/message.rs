use std::net::SocketAddr;

use bifrost::serialization::{Decode, Encode};

use super::stream::{WebSocketReceiver, WebSocketSender};

pub enum Message {
    Payload {
        address: SocketAddr,
        payload: Payload,
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
pub enum Payload {
    Text(String),
}

impl Payload {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }
}
