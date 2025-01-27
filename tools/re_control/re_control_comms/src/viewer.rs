use std::{
    collections::VecDeque,
    net::SocketAddrV4,
    sync::{Arc, RwLock},
    time::Duration,
};

use async_std::{net::TcpStream, sync::Mutex};
use bifrost::serialization::{Decode, Encode};
use futures::{
    channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
    io::{ReadHalf, WriteHalf},
    AsyncReadExt, AsyncWriteExt, StreamExt,
};
use miette::{IntoDiagnostic, Result};
use socket2::{Domain, Protocol, Socket, Type};
use tokio::sync::Notify;

use super::protocol::{HandlerFn, RobotMessage, ViewerMessage};

const LINGER_DURATION: Duration = Duration::from_secs(2);
const CONNECTION_ATTEMPT_DELAY: Duration = Duration::from_secs(5);
const CONNECTION_ATTEMPTS: usize = 3;

pub struct ControlViewer {
    address: SocketAddrV4,
    tx: UnboundedSender<ViewerMessage>,
    rx: Arc<Mutex<UnboundedReceiver<ViewerMessage>>>,
    message_queue: Arc<Mutex<VecDeque<ViewerMessage>>>,
    handlers: Arc<RwLock<Vec<HandlerFn<RobotMessage>>>>,
    notify: Arc<Notify>,
}

impl From<SocketAddrV4> for ControlViewer {
    fn from(address: SocketAddrV4) -> Self {
        let (tx, rx) = unbounded();
        Self {
            address,
            tx,
            rx: Arc::new(Mutex::new(rx)),
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
            handlers: Arc::new(RwLock::new(Vec::new())),
            notify: Arc::new(Notify::new()),
        }
    }
}

impl ControlViewer {
    #[must_use]
    pub fn run(self) -> ControlViewerHandle {
        // Spawn a background task to handle messages from the global channel.
        {
            let rx = Arc::clone(&self.rx);
            let message_queue = Arc::clone(&self.message_queue);
            let notify = Arc::clone(&self.notify);
            tokio::spawn(async move {
                Self::global_message_handler(rx, message_queue, notify).await;
            });
        }

        let app = Arc::new(self);
        let handle = app.clone();

        tokio::spawn(async move {
            for attempt in 1..=CONNECTION_ATTEMPTS {
                let socket = Socket::new(
                    Domain::for_address(app.address.into()),
                    Type::STREAM,
                    Some(Protocol::TCP),
                )
                .expect("failed to create viewer socket!");

                match socket.connect(&app.address.into()) {
                    Ok(()) => {
                        socket
                            .set_linger(Some(LINGER_DURATION))
                            .expect("Failed to set linger for socket");

                        let stream: std::net::TcpStream = socket.into();
                        let stream: async_std::net::TcpStream = stream.into();
                        app.handle_connection(stream).await;
                    }
                    Err(error) => {
                        tracing::error!(
                            ?error,
                            "failed to connect to {}, attempt [{}/{}]",
                            app.address,
                            attempt,
                            CONNECTION_ATTEMPTS
                        );
                    }
                }

                // Wait some time before attempting to reconnect
                tokio::time::sleep(CONNECTION_ATTEMPT_DELAY).await;
            }
        });

        ControlViewerHandle { app: handle }
    }

    async fn handle_connection(&self, socket: TcpStream) {
        let (read_half, write_half) = socket.split();

        // Spawn tasks to handle read and write
        let handlers = Arc::clone(&self.handlers);
        let reader_task = tokio::spawn(Self::handle_read(read_half, handlers));
        let writer_task = {
            let message_queue = Arc::clone(&self.message_queue);
            let notify = Arc::clone(&self.notify);
            tokio::spawn(async move {
                Self::handle_write(write_half, message_queue, notify).await;
            })
        };

        // Wait for the reader to complete. This happens when the TCP
        // connection ends or there was an error in the reader_task
        if let Err(e) = reader_task.await {
            tracing::error!(?e, "reader task ended");
        }
        // There is no reason to keep the writer task going when the reader
        // is completed.
        writer_task.abort();

        tracing::warn!("connection terminated with app: {}", self.address);
    }

    async fn global_message_handler(
        rx: Arc<Mutex<UnboundedReceiver<ViewerMessage>>>,
        message_queue: Arc<Mutex<VecDeque<ViewerMessage>>>,
        notify: Arc<Notify>,
    ) {
        let mut rx_guard = rx.lock().await;
        while let Some(message) = rx_guard.next().await {
            // Store the message in the queue and notify the writer task
            {
                let mut queue_guard = message_queue.lock().await;
                queue_guard.push_back(message);
            }
            notify.notify_one();
        }
        tracing::debug!("Global message channel closed");
    }

    async fn handle_read(
        mut read: ReadHalf<TcpStream>,
        handlers: Arc<RwLock<Vec<HandlerFn<RobotMessage>>>>,
    ) {
        let mut buf = [0; 1024];

        loop {
            // Read bytes received from the stream into a buffer. It is
            // possible that there are multiple message in the buffer.
            match read.read(&mut buf).await {
                Ok(0) => {
                    tracing::info!("Server closed connection");
                    break;
                }
                Ok(n) => {
                    // Fails when received too many bytes at once. Message
                    // might have been too big and got cut off.
                    assert_ne!(n, buf.len());
                    // Keep track of the amount of bytes that have been read
                    let mut bytes_read = 0;
                    // Keep decoding bytes to messages until we read the whole
                    // buffer
                    while bytes_read < n {
                        let message = RobotMessage::decode(&buf[bytes_read..n])
                            .expect("Failed to decode message");

                        let handlers = handlers.read().expect("failed to get reader");
                        for handler in handlers.iter() {
                            handler(&message);
                        }
                        // The decoded message length in bytes is the
                        // same as the encode length
                        bytes_read += message.encode_len();
                    }
                }
                Err(error) => {
                    tracing::error!(?error, "Error reading from server");
                    break;
                }
            }
        }
    }

    async fn handle_write(
        mut write: WriteHalf<TcpStream>,
        message_queue: Arc<Mutex<VecDeque<ViewerMessage>>>,
        notify: Arc<Notify>,
    ) {
        loop {
            let message_option;
            {
                let mut queue_guard = message_queue.lock().await;
                message_option = queue_guard.pop_front();
            }

            let Some(message) = message_option else {
                // If no messages are available, wait for a new one to arrive
                notify.notified().await;
                continue;
            };

            let mut data = vec![];
            if message.encode(&mut data).is_ok() {
                if let Err(error) = write.write_all(&data).await {
                    tracing::error!(?message, ?error, "failed to send message");
                    break;
                }
            }
        }
    }

    pub fn add_handler(&self, handler: HandlerFn<RobotMessage>) -> Result<()> {
        let mut handlers = self
            .handlers
            .write()
            .map_err(|_| miette::miette!("Failed to lock handlers"))?;
        handlers.push(handler);
        Ok(())
    }
}

#[derive(Clone)]
pub struct ControlViewerHandle {
    app: Arc<ControlViewer>,
}

impl ControlViewerHandle {
    #[must_use]
    pub fn addr(&self) -> SocketAddrV4 {
        self.app.address
    }

    pub fn send(&self, msg: ViewerMessage) -> Result<()> {
        self.app.tx.unbounded_send(msg).into_diagnostic()
    }

    pub fn add_handler<H>(&mut self, handler: H) -> Result<()>
    where
        H: Fn(&RobotMessage) + Send + Sync + 'static,
    {
        self.app.add_handler(Box::new(handler))
    }
}
