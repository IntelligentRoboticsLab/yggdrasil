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
use miette::{miette, IntoDiagnostic, Result};
use tokio::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
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

        let sender = Sender { tx, address };

        let receiver = Receiver { rx, address };

        Ok((sender, receiver))
    }
}

type WsSender = SplitSink<WebSocketStream<TcpStream>, tungstenite::Message>;
type WsReceiver = SplitStream<WebSocketStream<TcpStream>>;

/// A handle to send messages to a WebSocket stream.
#[derive(Debug)]
pub struct Sender {
    tx: WsSender,
    pub address: SocketAddr,
}

impl Sender {
    pub async fn send(&self, payload: Payload) -> Result<()> {
        let mut buf = Vec::with_capacity(payload.encode_len());
        payload.encode(&mut buf).into_diagnostic()?;

        // TEMP
        self.tx
            .send(tungstenite::Message::Text(String::from("hiiiiiiii")))
            .await
            .into_diagnostic()?;

        self.tx
            .send(tungstenite::Message::Binary(buf))
            .await
            .into_diagnostic()
    }
}

impl Clone for Sender {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx,
            address: self.address,
        }
    }
}

/// A handle to receive messages from a WebSocket stream.
#[derive(Debug)]
pub struct Receiver {
    pub address: SocketAddr,
    rx: WsReceiver,
}

impl Receiver {
    pub async fn next(&mut self) -> Result<Message> {
        let msg = self
            .rx
            .next()
            .await
            .ok_or_else(|| miette!("No more messages in the stream"))?
            .into_diagnostic()?;

        let payload = Payload::decode(msg.into_data().as_slice()).into_diagnostic()?;

        Ok(Message {
            address: self.address,
            payload,
        })
    }
}

/// A map that holds the [`Sender`]s for each connected [`SocketAddr`].
#[derive(Debug)]
pub struct Connections {
    tx: UnboundedSender<Message>,
    rx: UnboundedReceiver<Message>,
    map: HashMap<SocketAddr, Sender>,
}

impl Connections {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            rx,
            map: HashMap::new(),
        }
    }

    pub fn get(&self, address: SocketAddr) -> Option<&Sender> {
        self.map.get(&address)
    }

    pub fn insert(&mut self, connection: Sender) -> Option<Sender> {
        self.map.insert(connection.address, connection)
    }

    pub fn remove(&mut self, address: SocketAddr) -> Option<Sender> {
        self.map.remove(&address)
    }

    pub fn values(&self) -> Values<SocketAddr, Sender> {
        self.map.values()
    }

    pub fn values_mut(&mut self) -> ValuesMut<SocketAddr, Sender> {
        self.map.values_mut()
    }
}
