use std::net::SocketAddr;

use bifrost::serialization::{Decode, Encode};

#[derive(Debug)]
pub struct Message {
    pub address: SocketAddr,
    pub payload: Payload,
}

#[derive(Debug, Encode, Decode)]
pub enum Payload {
    Ping,
    Pong,
    Text(String),
}

impl Payload {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }
}
