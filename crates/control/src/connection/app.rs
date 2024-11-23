use std::collections::HashMap;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use async_std::net::{TcpListener, TcpStream};
use async_std::sync::Mutex;
use bevy::prelude::Resource;
use bevy::tasks::IoTaskPool;
use bifrost::serialization::{Decode, Encode};
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::io::{ReadHalf, WriteHalf};
use futures::{AsyncReadExt, AsyncWriteExt, StreamExt};
use miette::{IntoDiagnostic, Result};
use uuid::Uuid;

/// `T` Send Message type \
/// `U` Receive Message type
pub struct ControlApp<T, U>
where
    T: Encode,
    U: Decode,
{
    listener: TcpListener,
    handlers: RwLock<Vec<UnboundedSender<U>>>,
    clients: Arc<Mutex<HashMap<Uuid, UnboundedSender<T>>>>,
}

impl<T, U> ControlApp<T, U>
where
    T: Encode + Send + Clone + 'static,
    U: Decode + Debug + Send + Clone + 'static,
{
    pub async fn bind(addr: SocketAddr) -> Result<Self> {
        let listener = TcpListener::bind(addr).await.into_diagnostic()?;
        Ok(Self {
            listener,
            handlers: RwLock::new(Vec::new()),
            clients: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    pub fn run(self) -> ControlAppHandle<T, U> {
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
        {
            self.clients.lock().await.insert(Uuid::new_v4(), tx.clone());
        }

        // Spawn reader and writer tasks
        let reader_task = self.handle_reader(read_half);
        let writer_task = self.handle_writer(write_half, rx);

        let _ = futures::join!(reader_task, writer_task);

        // Remove the client when the connection ends
        {
            let mut clients = self.clients.lock().await;
            clients.retain(|_id, x| !x.same_receiver(&tx));
        }
    }

    async fn handle_reader(&self, mut read_half: ReadHalf<TcpStream>) {
        let mut buf = [0; 1024];
        loop {
            match read_half.read(&mut buf).await {
                Ok(0) => {
                    tracing::info!("Connection closed by client");
                    break;
                }
                Ok(n) => match &U::decode(&buf[..n]) {
                    Ok(msg) => {
                        tracing::info!("Received message: {:?}", msg);

                        let handlers = &mut self.handlers.read().expect("failed to get reader");

                        for handler in handlers.iter() {
                            handler.unbounded_send(msg.clone()).expect("Failed to send message");
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
        &self,
        mut write_half: WriteHalf<TcpStream>,
        mut rx: UnboundedReceiver<T>,
    ) {
        while let Some(message) = rx.next().await {
            // if message.is_disconnected() {
            //     tracing::info!("Received disconnect message, closing connection");
            //     break;
            // }

            // Encode and send response
            let mut data = vec![];
            match message.encode(&mut data) {
                Ok(_) => {
                    tracing::info!("Writing all info");
                    if write_half.write_all(&data).await.is_err() {
                        tracing::error!("Failed to send response to client");
                        break;
                    }
                }
                Err(e) => tracing::error!("Failed to encode message: {}", e)
            }
        }
    }

    pub fn add_handler(
        &self,
        handler: UnboundedSender<U>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut handlers = self
            .handlers
            .write()
            .map_err(|_| "Failed to lock handlers")?;
        handlers.push(handler);
        Ok(())
    }

    // pub async fn broadcast(&self, message: T) -> Result<()>
    // where
    //     T: Message,
    // {
    //     let clients = self.clients.lock().await;
    //     clients.iter().for_each(|mut client| {
    //         let _ = client.send(message.clone());
    //     });

    //     Ok(())
    // }
}

#[derive(Resource, Clone)]
pub struct ControlAppHandle<T, U>
where
    T: Encode,
    U: Decode,
{
    app: Arc<ControlApp<T, U>>,
}

impl<T, U> ControlAppHandle<T, U>
where
    T: Encode + Send + Clone + 'static,
    U: Decode + Debug + Send + Clone + 'static,
{
    pub fn add_handler(
        &mut self,
        handler: UnboundedSender<U>,
    ) -> std::result::Result<(), Box<dyn std::error::Error>> {
        self.app.add_handler(handler)
    }

    pub async fn broadcast(&self, message: T) {
        let clients = self.app.clients.lock().await;

        clients.iter().for_each(|(_id, client)| {
            tracing::info!("Sending message to a client");
            client.unbounded_send(message.clone()).expect("Failed to send message");
        });
    }

    pub async fn send(&self, message: T, client_id: Uuid) {
        let clients = self.app.clients.lock().await;

        let Some(client) = clients.get(&client_id) else {
            tracing::error!("Client does not exist");
            return
        };

        client.unbounded_send(message).expect("Failed to send message")
    }
}
