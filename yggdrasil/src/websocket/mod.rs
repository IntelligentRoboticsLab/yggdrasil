mod message;
mod stream;

use std::net::SocketAddr;

use miette::{IntoDiagnostic, Result};
use tokio::sync::mpsc::error::TryRecvError;
use tyr::{
    prelude::*,
    tasks::{
        asynchronous::{AsyncDispatcher, AsyncTask, AsyncTaskMap, AsyncTaskSet},
        task::{Dispatcher, Pollable, TaskResource},
    },
};

pub use message::{Message, Payload};
pub use stream::{
    WebSocketReceiver as Receiver, WebSocketSender as Sender, WebSocketServer,
    WebSocketServerHandle,
};

pub const ADDR: &str = "0.0.0.0:1984";

pub struct WebSocketModule;

impl Module for WebSocketModule {
    fn initialize(self, app: App) -> Result<App> {
        use crate::nao;

        Ok(app
            .add_startup_system(init_server)?
            .add_system(accept_sockets)
            .add_task::<AsyncTask<Result<AcceptCompleted>>>()?
            .add_system(handle_messages)
            .add_task::<AsyncTaskMap<SocketAddr, Result<RecvCompleted>>>()?
            .add_task::<AsyncTaskSet<Result<SendCompleted>>>()?
            .add_system(send_funny_message.after(nao::write_hardware_info)))
    }
}

fn init_server(storage: &mut Storage) -> Result<()> {
    let server = storage.map_resource_ref(|ad: &AsyncDispatcher| {
        ad.handle().block_on(WebSocketServer::bind(ADDR))
    })?;

    tracing::info!("Started WebSocket server, listening on {ADDR}");

    storage.add_resource(Resource::new(server))?;

    Ok(())
}

struct AcceptCompleted;

#[system]
fn accept_sockets(
    server: &WebSocketServer,
    task: &mut AsyncTask<Result<AcceptCompleted>>,
) -> Result<()> {
    let _ = task.try_spawn({
        let server = server.handle().clone();
        async move {
            loop {
                server.accept().await?
            }
        }
    });

    Ok(())
}

#[system]
fn handle_messages(
    server: &mut WebSocketServer,
    receive_tasks: &mut AsyncTaskMap<SocketAddr, Result<RecvCompleted>>,
    send_tasks: &mut AsyncTaskSet<Result<SendCompleted>>,
) -> Result<()> {
    // Handle any errors with completed tasks
    receive_tasks
        .poll()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    send_tasks
        .poll()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

    // Receive new messages
    loop {
        match server.rx.try_recv() {
            Ok(msg) => handle_message(msg, &mut server, &mut receive_tasks)?,
            Err(TryRecvError::Empty) => break,
            Err(e) => return Err(e).into_diagnostic(),
        }
    }

    Ok(())
}

fn handle_message(
    msg: Message,
    server: &mut WebSocketServer,
    receive_tasks: &mut AsyncTaskMap<SocketAddr, Result<RecvCompleted>>,
) -> Result<()> {
    match msg {
        Message::Payload { address, payload } => {
            println!("Received message `{:?}` from address {}", payload, address);
        }
        Message::OpenConnection { tx, rx, address } => {
            tracing::info!("Opened connection with {address}");
            server.connections.insert(address, tx);
            receive_tasks.try_spawn(address, receive_messages(rx))?;
        }
        Message::CloseConnection { address } => {
            tracing::info!("Closed connection with {address}");
            server.connections.remove(&address);
        }
    };

    Ok(())
}

#[system]
fn send_funny_message(
    server: &WebSocketServer,
    send_tasks: &mut AsyncTaskSet<Result<SendCompleted>>,
) -> Result<()> {
    for tx in server.connections.values() {
        send_tasks.spawn(send_message(tx.clone(), Payload::text("🦀🦀🦀")));
    }

    Ok(())
}

struct RecvCompleted;

async fn receive_messages(mut rx: Receiver) -> Result<RecvCompleted> {
    // Keep receiving messages
    while let Some(payload) = rx.recv().await? {
        let msg = Message::Payload {
            address: rx.address,
            payload,
        };

        rx.message_queue_tx.send(msg).into_diagnostic()?;
    }

    // No more messages, close the connection
    let close_msg = Message::CloseConnection {
        address: rx.address,
    };

    rx.message_queue_tx.send(close_msg).into_diagnostic()?;

    Ok(RecvCompleted)
}

struct SendCompleted;

async fn send_message(mut tx: Sender, payload: Payload) -> Result<SendCompleted> {
    tx.send(payload).await?;

    Ok(SendCompleted)
}
