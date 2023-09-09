use std::{
    collections::{
        hash_map::{Values, ValuesMut},
        HashMap,
    },
    net::SocketAddr,
    sync::Arc,
};

use bifrost::serialization::Decode;
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

use super::message::Payload;

#[derive(Debug, Clone)]
pub struct WebSocketListener(Arc<TcpListener>);

impl WebSocketListener {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> Result<Self> {
        Ok(Self(Arc::new(
            TcpListener::bind(addr).await.into_diagnostic()?,
        )))
    }

    pub async fn accept(&self) -> Result<(WebSocketSender, WebSocketReceiver)> {
        let (stream, address) = self.0.accept().await.into_diagnostic()?;

        let socket = tokio_tungstenite::accept_async(stream)
            .await
            .into_diagnostic()?;

        let (tx, rx) = socket.split();

        let sender = WebSocketSender {
            tx: Arc::new(Mutex::new(tx)),
            address,
        };

        let receiver = WebSocketReceiver { rx };

        Ok((sender, receiver))
    }
}

type Tx = SplitSink<WebSocketStream<TcpStream>, tungstenite::Message>;
type Rx = SplitStream<WebSocketStream<TcpStream>>;

#[derive(Debug)]
pub struct WebSocketSender {
    tx: Arc<Mutex<Tx>>,
    pub address: SocketAddr,
}

impl WebSocketSender {
    pub async fn send(&self, buf: Vec<u8>) -> Result<()> {
        self.tx
            .lock()
            .await
            .send(tungstenite::Message::Binary(buf))
            .await
            .into_diagnostic()
    }
}

impl Clone for WebSocketSender {
    fn clone(&self) -> Self {
        Self {
            tx: Arc::clone(&self.tx),
            address: self.address,
        }
    }
}

#[derive(Debug)]
pub struct WebSocketReceiver {
    rx: Rx,
}

impl WebSocketReceiver {
    pub async fn recv_next(&mut self) -> Result<Option<Payload>> {
        let Some(msg) = self.rx.next().await.transpose().into_diagnostic()? else {
            return Ok(None);
        };

        // We only make use of bifrost encoded messages through the `Payload` type
        if !msg.is_binary() {
            bail!("Received non-binary message: `{msg:?}`");
        }

        let payload = Payload::decode(msg.into_data().as_slice()).into_diagnostic()?;

        Ok(Some(payload))
    }
}

#[derive(Debug, Default)]
pub struct Connections(HashMap<SocketAddr, WebSocketSender>);

impl Connections {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, address: SocketAddr) -> Option<&WebSocketSender> {
        self.0.get(&address)
    }

    pub fn insert(&mut self, connection: WebSocketSender) -> Option<WebSocketSender> {
        self.0.insert(connection.address, connection)
    }

    pub fn remove(&mut self, address: SocketAddr) -> Option<WebSocketSender> {
        self.0.remove(&address)
    }

    pub fn values(&self) -> Values<SocketAddr, WebSocketSender> {
        self.0.values()
    }

    pub fn values_mut(&mut self) -> ValuesMut<SocketAddr, WebSocketSender> {
        self.0.values_mut()
    }
}
