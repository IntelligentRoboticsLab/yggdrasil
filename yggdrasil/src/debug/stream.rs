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

use super::message::{DebugPayload, Message};

pub type MessageQueueSender = UnboundedSender<Message>;
pub type MessageQueueReceiver = UnboundedReceiver<Message>;

type RawWebSocketSender = SplitSink<WebSocketStream<TcpStream>, tungstenite::Message>;
type RawWebSocketReceiver = SplitStream<WebSocketStream<TcpStream>>;

pub struct WebSocketServer {
    handle: WebSocketServerHandle,
    pub rx: MessageQueueReceiver,
    pub connections: HashMap<SocketAddr, WebSocketSender>,
}

// Clonable handle so we can accept from other threads
pub struct WebSocketServerHandle {
    listener: Arc<TcpListener>,
    tx: MessageQueueSender,
}

impl Clone for WebSocketServerHandle {
    fn clone(&self) -> Self {
        Self {
            listener: Arc::clone(&self.listener),
            tx: self.tx.clone(),
        }
    }
}

impl WebSocketServer {
    pub async fn bind<A: ToSocketAddrs>(address: A) -> Result<Self> {
        let listener = Arc::new(TcpListener::bind(address).await.into_diagnostic()?);
        let connections = HashMap::new();
        let (tx, rx) = mpsc::unbounded_channel::<Message>();

        let handle = WebSocketServerHandle { listener, tx };

        Ok(Self {
            handle,
            connections,
            rx,
        })
    }

    pub fn handle(&self) -> &WebSocketServerHandle {
        &self.handle
    }

    pub async fn accept(&self) -> Result<()> {
        self.handle.accept().await
    }
}

impl WebSocketServerHandle {
    pub async fn accept(&self) -> Result<()> {
        let (stream, address) = self.listener.accept().await.into_diagnostic()?;
        let socket = tokio_tungstenite::accept_async(stream)
            .await
            .into_diagnostic()?;

        let (raw_ws_tx, raw_ws_rx) = socket.split();

        let tx = WebSocketSender {
            raw: Arc::new(Mutex::new(raw_ws_tx)),
            address,
        };

        let rx = WebSocketReceiver {
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

pub struct WebSocketSender {
    raw: Arc<Mutex<RawWebSocketSender>>,
    pub address: SocketAddr,
}

impl WebSocketSender {
    pub async fn send(&mut self, payload: DebugPayload) -> Result<()> {
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

impl Clone for WebSocketSender {
    fn clone(&self) -> Self {
        Self {
            raw: Arc::clone(&self.raw),
            address: self.address,
        }
    }
}

pub struct WebSocketReceiver {
    websocket_rx: RawWebSocketReceiver,
    pub address: SocketAddr,
    pub message_queue_tx: MessageQueueSender,
}

impl WebSocketReceiver {
    pub async fn recv(&mut self) -> Result<Option<DebugPayload>> {
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
            let payload = DebugPayload::decode(bytes.as_slice()).into_diagnostic()?;

            return Ok(Some(payload));
        }

        // no more messages, we should close the connection
        Ok(None)
    }
}
