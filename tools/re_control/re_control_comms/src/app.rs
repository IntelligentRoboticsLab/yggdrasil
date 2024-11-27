use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use async_std::net::TcpStream;
use async_std::sync::Mutex;
use async_std::task::spawn;
use bevy::prelude::Resource;
use bevy::tasks::IoTaskPool;
use bifrost::serialization::{Decode, Encode};
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::io::{ReadHalf, WriteHalf};
use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use miette::{IntoDiagnostic, Result};
use socket2::{Domain, Protocol, Socket, Type};
use uuid::Uuid;

use super::protocol::{RobotMessage, ViewerMessage};

const LISTEN_BACKLOG: i32 = 1024;

pub struct NotifyConnection {
    pub id: Uuid,
}

pub struct ControlApp {
    listener: async_std::net::TcpListener,
    handlers: Arc<RwLock<Vec<UnboundedSender<ViewerMessage>>>>,
    clients: Arc<Mutex<HashMap<Uuid, UnboundedSender<RobotMessage>>>>,
    new_connection_notifyer: UnboundedSender<NotifyConnection>,
}

impl ControlApp {
    pub fn bind(
        addr: SocketAddr,
        new_connection_notifyer: UnboundedSender<NotifyConnection>,
    ) -> Result<Self> {
        let socket =
            Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP)).into_diagnostic()?;
        socket.set_reuse_address(true).into_diagnostic()?;

        // Bind the socket to the given addr
        socket.bind(&addr.into()).into_diagnostic()?;

        // Set socket as a listener socket
        socket.listen(LISTEN_BACKLOG).into_diagnostic()?;

        // Transforming the `socket2::socket::Socket` to a `async_std::net::TcpListener`
        let listener = std::net::TcpListener::from(socket);
        let listener = async_std::net::TcpListener::from(listener);

        Ok(Self {
            listener,
            handlers: Arc::new(RwLock::new(Vec::new())),
            clients: Arc::new(Mutex::new(HashMap::new())),
            new_connection_notifyer,
        })
    }

    pub fn run(self) -> ControlAppHandle {
        tracing::info!(
            "Control app is running on {:?}",
            self.listener.local_addr().unwrap()
        );

        let app = Arc::new(self);
        let handle = app.clone();

        let io = IoTaskPool::get();
        io.spawn(async move {
            loop {
                match app.listener.accept().await {
                    Ok((socket, addr)) => {
                        tracing::info!("Connection with a new client: {:?}", addr);

                        let app = Arc::clone(&app);
                        io.spawn(async move {
                            app.handle_connection(socket).await;
                        })
                        .detach();
                    }
                    Err(e) => {
                        tracing::error!("Failed to connect with client: {:?}", e);
                    }
                }
            }
        })
        .detach();

        ControlAppHandle { app: handle }
    }

    async fn handle_connection(&self, socket: TcpStream) {
        let client_addr = socket
            .peer_addr()
            .expect("Failed to get peer address from socket");

        let (read_half, write_half) = socket.split();
        let (tx, rx) = mpsc::unbounded();

        // Add the client to the list
        let id = Uuid::new_v4();
        {
            self.clients.lock().await.insert(id, tx.clone());
        }
        tracing::info!("Number of clients: {}", self.clients.lock().await.len());

        // Spawn reader and writer tasks
        let handlers = Arc::clone(&self.handlers);
        let reader_task = spawn(async { Self::handle_reader(read_half, handlers).await });
        let _writer_task = spawn(async { Self::handle_writer(write_half, rx).await });

        // Notify to a bevy system that a new connection is made
        let msg = NotifyConnection { id };
        self.new_connection_notifyer
            .unbounded_send(msg)
            .expect("Failed to send message");

        // Only need to wait until the reader_task is done.
        // reader_task is done when the connection ends. The writer task
        // should also stop at that moment.
        reader_task.await;

        // Remove the client when the connection ends. Removing the client
        // will also stop the writer_task.
        {
            let mut clients = self.clients.lock().await;
            clients.retain(|_id, x| !x.same_receiver(&tx));
        }

        tracing::info!("Connection closed by client at {client_addr}");
    }

    async fn handle_reader(
        mut read_half: ReadHalf<TcpStream>,
        handlers: Arc<RwLock<Vec<UnboundedSender<ViewerMessage>>>>,
    ) {
        let mut buf = [0; 1024];
        loop {
            // Read bytes received from the stream into a buffer. It is
            // possible that there are multiple message in the buffer.
            match read_half.read(&mut buf).await {
                Ok(0) => {
                    break;
                }
                Ok(n) => {
                    // Keep track of the amount of bytes that have been read
                    let mut bytes_read = 0;
                    // Keep decoding bytes to messages until we read the whole
                    // buffer
                    while bytes_read < n {
                        match &ViewerMessage::decode(&buf[..n]) {
                            Ok(message) => {
                                let handlers = handlers.read().expect("failed to lock handlers");

                                for handler in handlers.iter() {
                                    handler
                                        .unbounded_send(message.clone())
                                        .expect("Failed to send message");
                                }
                                // The decoded message length in bytes is the
                                // same as the encode length
                                bytes_read += message.encode_len();
                            }
                            Err(e) => {
                                tracing::error!("Failed to decode message: {:?}", e);
                            }
                        }
                    }
                }
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
            // Encode and send response
            let mut data = vec![];
            match message.encode(&mut data) {
                Ok(()) => {
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
            .unbounded_send(message.clone())
            .expect("Failed to send message");
    }
}
