use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use async_std::net::{TcpListener, TcpStream};
use async_std::sync::Mutex;
use async_std::task::spawn;
use bevy::prelude::Resource;
use bevy::tasks::IoTaskPool;
use bifrost::serialization::{Decode, Encode};
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::io::{ReadHalf, WriteHalf};
use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use miette::{IntoDiagnostic, Result};
use uuid::Uuid;

use super::protocol::{RobotMessage, ViewerMessage};

pub struct NotifyConnection {
    pub id: Uuid,
}

pub struct ControlApp {
    listener: TcpListener,
    handlers: Arc<RwLock<Vec<UnboundedSender<ViewerMessage>>>>,
    clients: Arc<Mutex<HashMap<Uuid, UnboundedSender<RobotMessage>>>>,
    new_connection_notifyer: UnboundedSender<NotifyConnection>,
}

impl ControlApp {
    pub async fn bind(
        addr: SocketAddr,
        new_clients: UnboundedSender<NotifyConnection>,
    ) -> Result<Self> {
        let listener = TcpListener::bind(addr).await.into_diagnostic()?;
        Ok(Self {
            listener,
            handlers: Arc::new(RwLock::new(Vec::new())),
            clients: Arc::new(Mutex::new(HashMap::new())),
            new_connection_notifyer: new_clients,
        })
    }

    pub fn run(self) -> ControlAppHandle {
        tracing::info!(
            "Server running on {:?}",
            self.listener.local_addr().unwrap()
        );

        let app = Arc::new(self);
        let handle = app.clone();

        let io = IoTaskPool::get();
        io.spawn(async move {
            match app.listener.accept().await {
                Ok((socket, addr)) => {
                    tracing::info!("Connection with a new client: {:?}", addr);

                    io.spawn(async move {
                        app.handle_connection(socket).await;
                    })
                    .detach();
                }
                Err(e) => {
                    tracing::error!("Failed to connect with client: {:?}", e)
                }
            }
        })
        .detach();

        ControlAppHandle { app: handle }
    }

    async fn handle_connection(&self, socket: TcpStream) {
        let (read_half, write_half) = socket.split();
        let (tx, rx) = mpsc::unbounded();

        // Add the client to the list
        let id = Uuid::new_v4();
        {
            self.clients.lock().await.insert(id, tx.clone());
        }

        // Spawn reader and writer tasks
        let handlers = Arc::clone(&self.handlers);
        let reader_task = spawn(async { Self::handle_reader(read_half, handlers).await });
        let writer_task = spawn(async { Self::handle_writer(write_half, rx).await });

        // Notify to a bevy system that a new connection is made
        let msg = NotifyConnection { id };
        self.new_connection_notifyer
            .unbounded_send(msg)
            .expect("Failed to send message");

        let _ = futures::join!(reader_task, writer_task);

        // Remove the client when the connection ends
        {
            let mut clients = self.clients.lock().await;
            clients.retain(|_id, x| !x.same_receiver(&tx));
        }
    }

    async fn handle_reader(
        mut read_half: ReadHalf<TcpStream>,
        handlers: Arc<RwLock<Vec<UnboundedSender<ViewerMessage>>>>,
    ) {
        let mut buf = [0; 1024];
        loop {
            match read_half.read(&mut buf).await {
                Ok(0) => {
                    tracing::info!("Connection closed by client");
                    break;
                }
                Ok(n) => match &ViewerMessage::decode(&buf[..n]) {
                    Ok(msg) => {
                        let handlers = handlers.read().expect("failed to lock handlers");

                        for handler in handlers.iter() {
                            handler
                                .unbounded_send(msg.clone())
                                .expect("Failed to send message");
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to decode message: {:?}", e);
                    }
                },
                Err(e) => {
                    tracing::error!("Error reading from socket: {:?}", e);
                    break;
                }
            }
        }
    }

    async fn handle_writer(
        mut write_half: WriteHalf<TcpStream>,
        mut rx: UnboundedReceiver<RobotMessage>,
    ) {
        while let Some(message) = rx.next().await {
            if matches!(message, RobotMessage::Disconnect) {
                break;
            }

            // Encode and send response
            let mut data = vec![];
            match message.encode(&mut data) {
                Ok(_) => {
                    if write_half.write_all(&data).await.is_err() {
                        tracing::error!("Failed to send response to client");
                        break;
                    }
                }
                Err(e) => tracing::error!("Failed to encode message: {}", e),
            }
        }
    }

    pub fn add_handler(
        &self,
        handler: UnboundedSender<ViewerMessage>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut handlers = self
            .handlers
            .write()
            .map_err(|_| "Failed to lock handlers")?;
        handlers.push(handler);
        Ok(())
    }
}

#[derive(Resource, Clone)]
pub struct ControlAppHandle {
    app: Arc<ControlApp>,
}

impl ControlAppHandle {
    pub fn add_handler(
        &mut self,
        handler: UnboundedSender<ViewerMessage>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        self.app.add_handler(handler)
    }

    /// Send a `RobotMessage` to all connected clients
    pub async fn broadcast(&self, message: RobotMessage) {
        let clients = self.app.clients.lock().await;

        clients.iter().for_each(|(_id, client)| {
            client
                .unbounded_send(message.clone())
                .expect("Failed to send message");
        });
    }

    /// Send a `RobotMessage` to a specific connected client
    pub async fn send(&self, message: RobotMessage, client_id: Uuid) {
        let clients = self.app.clients.lock().await;

        let Some(client) = clients.get(&client_id) else {
            tracing::error!("Client does not exist");
            return;
        };

        client
            .unbounded_send(message)
            .expect("Failed to send message")
    }
}
