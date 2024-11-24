use std::{
    collections::VecDeque,
    net::SocketAddrV4,
    sync::{Arc, RwLock},
    time::Duration,
};

use async_std::{net::TcpStream, sync::Mutex};
use bifrost::serialization::{Decode, Encode};
use futures::{
    channel::mpsc::{unbounded, TrySendError, UnboundedReceiver, UnboundedSender},
    io::{ReadHalf, WriteHalf},
    AsyncReadExt, AsyncWriteExt, StreamExt,
};
use tokio::sync::Notify;

use super::protocol::{HandlerFn, RobotMessage, ViewerMessage};

pub struct ControlViewer {
    address: SocketAddrV4,
    tx: UnboundedSender<ViewerMessage>,
    rx: Arc<Mutex<UnboundedReceiver<ViewerMessage>>>,
    message_queue: Arc<Mutex<VecDeque<ViewerMessage>>>,
    handlers: Arc<RwLock<Vec<HandlerFn<RobotMessage>>>>,
    notify: Arc<Notify>,
}

impl ControlViewer {
    pub async fn connect(address: SocketAddrV4) -> tokio::io::Result<Self> {
        let (tx, rx) = unbounded();
        Ok(Self {
            address,
            tx,
            rx: Arc::new(Mutex::new(rx)),
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
            handlers: Arc::new(RwLock::new(Vec::new())),
            notify: Arc::new(Notify::new()),
        })
    }

    pub async fn run(self) -> ControlViewerHandle {
        tracing::info!("Starting client");

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
            loop {
                match TcpStream::connect(app.address).await {
                    Ok(socket) => {
                        app.handle_connection(socket).await;
                    }
                    Err(e) => {
                        tracing::error!("Failed to connect to {}: {:?}", app.address, e);
                    }
                }
                // Wait some time before attempting to reconnect
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        ControlViewerHandle { app: handle }
    }

    async fn handle_connection(&self, socket: TcpStream) {
        // if let Err(e) = socket.set_linger(Some(Duration::from_secs(2))) {
        //     tracing::error!("Failed to set socket linger: {:?}", e);
        // }

        tracing::info!("Connected to {}", self.address);
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

        // Wait for tasks to complete
        tokio::select! {
            result = reader_task => {
                if let Err(e) = result {
                    tracing::error!("Reader task ended with error: {:?}", e);
                }
            }
            result = writer_task => {
                if let Err(e) = result {
                    tracing::error!("Writer task ended with error: {:?}", e);
                }
            }
        }

        tracing::info!("Connection lost. Attempting to reconnect...");
    }

    async fn global_message_handler(
        rx: Arc<Mutex<UnboundedReceiver<ViewerMessage>>>,
        message_queue: Arc<Mutex<VecDeque<ViewerMessage>>>,
        notify: Arc<Notify>,
    ) {
        let mut rx_guard = rx.lock().await;
        while let Some(message) = rx_guard.next().await {
            // Store the message in the queue and notify the writer task
            let mut queue_guard = message_queue.lock().await;
            queue_guard.push_back(message);
            drop(queue_guard);
            notify.notify_one();
        }
        tracing::info!("Global message channel closed");
    }

    async fn handle_read(
        mut read: ReadHalf<TcpStream>,
        handlers: Arc<RwLock<Vec<HandlerFn<RobotMessage>>>>,
    ) {
        let mut buf = [0; 1024];
        loop {
            match read.read(&mut buf).await {
                Ok(0) => {
                    tracing::info!("Server closed connection");
                    break;
                }
                Ok(n) => match RobotMessage::decode(&buf[..n]) {
                    Ok(message) => {
                        let handlers = handlers.read().expect("failed to get reader");
                        for handler in handlers.iter() {
                            handler(&message);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to decode message: {:?}", e);
                    }
                },
                Err(e) => {
                    tracing::error!("Error reading from server: {:?}", e);
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

            if matches!(message, ViewerMessage::Disconnect) {
                tracing::info!("Disconnecting...");
                break;
            }

            let mut data = vec![];
            if message.encode(&mut data).is_ok() {
                if let Err(e) = write.write_all(&data).await {
                    tracing::error!("Failed to send message: {:?}, error: {:?}", message, e);
                    break;
                }
            }
        }
    }

    pub fn add_handler(
        &self,
        handler: HandlerFn<RobotMessage>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut handlers = self
            .handlers
            .write()
            .map_err(|_| "Failed to lock handlers")?;
        handlers.push(handler);
        Ok(())
    }
}

#[derive(Clone)]
pub struct ControlViewerHandle {
    app: Arc<ControlViewer>,
}

impl ControlViewerHandle {
    pub fn send(&self, msg: ViewerMessage) -> Result<(), TrySendError<ViewerMessage>> {
        self.app.tx.unbounded_send(msg)
    }

    pub fn add_handler<H>(
        &mut self,
        handler: H,
    ) -> std::result::Result<(), Box<dyn std::error::Error>>
    where
        H: Fn(&RobotMessage) + Send + Sync + 'static,
    {
        self.app.add_handler(Box::new(handler))
    }
}
