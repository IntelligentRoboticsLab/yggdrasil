use std::{
    collections::{
        hash_map::{Values, ValuesMut},
        HashMap,
    },
    net::SocketAddr,
    sync::Arc,
};

use bifrost::serialization::{Decode, Encode};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use miette::{bail, IntoDiagnostic, Result};
use tokio::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::Mutex,
};
use tokio_tungstenite::{tungstenite, WebSocketStream};

use super::message::{Message, Payload};

#[derive(Debug, Clone)]
pub struct Listener(Arc<TcpListener>);

impl Listener {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        Ok(Self(Arc::new(
            TcpListener::bind(addr).await.into_diagnostic()?,
        )))
    }

    pub async fn accept(&self) -> Result<(Sender, Receiver)> {
        let (stream, address) = self.0.accept().await.into_diagnostic()?;

        let socket = tokio_tungstenite::accept_async(stream)
            .await
            .into_diagnostic()?;

        let (tx, rx) = socket.split();

        let sender = Sender {
            tx: Arc::new(Mutex::new(tx)),
            address,
        };

        let receiver = Receiver { rx, address };

        Ok((sender, receiver))
    }
}

type Tx = SplitSink<WebSocketStream<TcpStream>, tungstenite::Message>;
type Rx = SplitStream<WebSocketStream<TcpStream>>;

/// A handle to send messages to a WebSocket stream.
#[derive(Debug)]
pub struct Sender {
    tx: Arc<Mutex<Tx>>,
    pub address: SocketAddr,
}

impl Sender {
    pub async fn send(&self, payload: Payload) -> Result<()> {
        let mut buf = Vec::with_capacity(payload.encode_len());
        payload.encode(&mut buf).into_diagnostic()?;

        self.tx
            .lock()
            .await
            .send(tungstenite::Message::Binary(buf))
            .await
            .into_diagnostic()
    }
}

impl Clone for Sender {
    fn clone(&self) -> Self {
        Self {
            tx: Arc::clone(&self.tx),
            address: self.address,
        }
    }
}

/// A handle to receive messages from a WebSocket stream.
#[derive(Debug)]
pub struct Receiver {
    pub address: SocketAddr,
    rx: Rx,
}

impl Receiver {
    pub async fn recv_next(&mut self) -> Result<Option<Message>> {
        let Some(msg) = self.rx.next().await.transpose().into_diagnostic()? else {
            return Ok(None);
        };

        // We only make use of bifrost encoded messages through the `Payload` type
        if !msg.is_binary() {
            bail!("Received non-binary message: `{msg:?}`");
        }

        let payload = Payload::decode(msg.into_data().as_slice()).into_diagnostic()?;

        Ok(Some(Message {
            address: self.address,
            payload,
        }))
    }
}

/// A map that holds the [`Sender`]s for each connected [`SocketAddr`].
#[derive(Debug, Default)]
pub struct Connections(HashMap<SocketAddr, Sender>);

impl Connections {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, address: SocketAddr) -> Option<&Sender> {
        self.0.get(&address)
    }

    pub fn insert(&mut self, connection: Sender) -> Option<Sender> {
        self.0.insert(connection.address, connection)
    }

    pub fn remove(&mut self, address: SocketAddr) -> Option<Sender> {
        self.0.remove(&address)
    }

    pub fn values(&self) -> Values<SocketAddr, Sender> {
        self.0.values()
    }

    pub fn values_mut(&mut self) -> ValuesMut<SocketAddr, Sender> {
        self.0.values_mut()
    }
}
