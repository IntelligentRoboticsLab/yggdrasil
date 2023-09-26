use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use bifrost::serialization::{Decode, Encode};
use futures::{
    stream::{SplitSink, SplitStream},
    SinkExt, StreamExt,
};
use miette::{IntoDiagnostic, Result};
use tokio::{
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        Mutex,
    },
};
use tokio_tungstenite::{tungstenite, WebSocketStream};

use crate::websocket::message::Payload;

use super::message::Message;

pub type MessageQueueTx = UnboundedSender<Message>;
pub type MessageQueueRx = UnboundedReceiver<Message>;

type RawWebSocketTx = SplitSink<WebSocketStream<TcpStream>, tungstenite::Message>;
type RawWebSocketRx = SplitStream<WebSocketStream<TcpStream>>;

pub struct WebSocketServer {
    listener: Arc<TcpListener>,
    pub connections: HashMap<SocketAddr, WebSocketTx>,
    pub rx: MessageQueueRx,
    tx: MessageQueueTx,
}

pub struct WebSocketServerHandle {
    listener: Arc<TcpListener>,
    tx: MessageQueueTx,
}

impl WebSocketServer {
    pub async fn bind<A: ToSocketAddrs>(address: A) -> Result<Self> {
        let listener = Arc::new(TcpListener::bind(address).await.into_diagnostic()?);
        let connections = HashMap::new();
        let (tx, rx) = mpsc::unbounded_channel::<Message>();

        Ok(Self {
            listener,
            connections,
            rx,
            tx,
        })
    }

    pub fn handle(&self) -> WebSocketServerHandle {
        WebSocketServerHandle {
            listener: self.listener.clone(),
            tx: self.tx.clone(),
        }
    }
}

impl WebSocketServerHandle {
    pub async fn accept(&self) -> Result<()> {
        let (stream, address) = self.listener.accept().await.into_diagnostic()?;
        let socket = tokio_tungstenite::accept_async(stream)
            .await
            .into_diagnostic()?;

        let (raw_ws_tx, raw_ws_rx) = socket.split();

        let tx = WebSocketTx {
            raw: Arc::new(Mutex::new(raw_ws_tx)),
            address,
        };

        let rx = WebSocketRx {
            websocket_rx: raw_ws_rx,
            address,
            message_queue_tx: self.tx.clone(),
        };

        self.tx
            .send(Message::OpenConnection { tx, rx, address })
            .into_diagnostic()?;

        Ok(())
    }
}

pub struct WebSocketTx {
    raw: Arc<Mutex<RawWebSocketTx>>,
    pub address: SocketAddr,
}

impl WebSocketTx {
    pub async fn send(&mut self, payload: Payload) -> Result<()> {
        let mut buf = Vec::with_capacity(payload.encode_len());
        payload.encode(&mut buf).into_diagnostic()?;

        self.raw
            .lock()
            .await
            .send(tungstenite::Message::Binary(buf))
            .await
            .into_diagnostic()
    }
}

impl Clone for WebSocketTx {
    fn clone(&self) -> Self {
        Self {
            raw: Arc::clone(&self.raw),
            address: self.address,
        }
    }
}

pub struct WebSocketRx {
    websocket_rx: RawWebSocketRx,
    pub address: SocketAddr,
    pub message_queue_tx: MessageQueueTx,
}

impl WebSocketRx {
    pub async fn recv(&mut self) -> Result<Option<Payload>> {
        if let Some(msg) = self
            .websocket_rx
            .next()
            .await
            .transpose()
            .into_diagnostic()?
        {
            if msg.is_close() {
                return Ok(None);
            }

            let bytes = msg.into_data();
            let payload = Payload::decode(bytes.as_slice()).into_diagnostic()?;

            return Ok(Some(payload));
        }

        // no more messages, we should close the connection
        Ok(None)
    }
}
