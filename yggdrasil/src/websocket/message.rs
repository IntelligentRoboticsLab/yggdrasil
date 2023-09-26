use std::net::SocketAddr;

use bifrost::serialization::{Decode, Encode};

use super::stream::{WebSocketRx, WebSocketTx};

pub enum Message {
    Payload {
        address: SocketAddr,
        payload: Payload,
    },
    OpenConnection {
        tx: WebSocketTx,
        rx: WebSocketRx,
        address: SocketAddr,
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
