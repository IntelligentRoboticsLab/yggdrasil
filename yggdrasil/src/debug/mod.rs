mod message;

#[cfg(feature = "lola")]
mod stream;

use std::{net::{Ipv4Addr, SocketAddr}, sync::RwLockReadGuard};

use miette::{IntoDiagnostic, Result};

#[cfg(feature = "lola")]
use tokio::sync::mpsc::error::TryRecvError;

#[cfg(feature = "lola")]
use tyr::{
    prelude::*,
    tasks::{
        asynchronous::{AsyncDispatcher, AsyncTask, AsyncTaskMap, AsyncTaskSet},
        task::{Dispatcher, Pollable, TaskResource},
    },
    DebugView,
};

pub use message::DebugPayload;

#[cfg(feature = "lola")]
pub use message::Message;

#[cfg(feature = "lola")]
pub use stream::{
    WebSocketReceiver as Receiver, WebSocketSender as Sender, WebSocketServer,
    WebSocketServerHandle,
};

pub const PORT: u16 = 1984;
pub const ADDR: Ipv4Addr = Ipv4Addr::new(0, 0, 0, 0);

#[cfg(feature = "lola")]
pub struct WebSocketModule;

#[cfg(feature = "lola")]
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
            .add_debuggable_resource(Resource::new(Magic { data: "hello".to_string(), something: 10}))?
            .add_system(send_debuggables))
    }
}

#[derive(Debug)]
struct Magic {
    data: String,
    something: u32,
}

#[cfg(feature = "lola")]
fn init_server(storage: &mut Storage) -> Result<()> {
    let server = storage.map_resource_ref(|ad: &AsyncDispatcher| {
        ad.handle().block_on(WebSocketServer::bind((ADDR, PORT)))
    })??;

    tracing::info!("Started WebSocket server, listening on {ADDR}:{PORT}");

    storage.add_resource(Resource::new(server))?;

    Ok(())
}

#[cfg(feature = "lola")]
struct AcceptCompleted;

#[cfg(feature = "lola")]
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

#[cfg(feature = "lola")]
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
            Ok(msg) => handle_message(msg, server, receive_tasks)?,
            Err(TryRecvError::Empty) => break,
            Err(e) => return Err(e).into_diagnostic(),
        }
    }

    Ok(())
}

#[cfg(feature = "lola")]
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

struct DebuggableTraitObject<'a>(RwLockReadGuard<'a, dyn std::fmt::Debug + Send + Sync>);

impl<'a> std::fmt::Debug for DebuggableTraitObject<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[cfg(feature = "lola")]
#[system]
fn send_debuggables(
    server: &WebSocketServer,
    debug_view: &DebugView,
    send_tasks: &mut AsyncTaskSet<Result<SendCompleted>>,
) -> Result<()> {
    for tx in server.connections.values() {
        for resource in debug_view.resources() {
            let dbg = resource.read().unwrap();
    ;
            let data = format!("{:?}", x);

            send_tasks.spawn(send_message(
                tx.clone(),
                DebugPayload::Text(resource.0.to_string(), data),
            ));
        }
    }

    Ok(())
}

#[cfg(feature = "lola")]
struct RecvCompleted;

#[cfg(feature = "lola")]
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

#[cfg(feature = "lola")]
struct SendCompleted;

#[cfg(feature = "lola")]
async fn send_message(mut tx: Sender, payload: DebugPayload) -> Result<SendCompleted> {
    tx.send(payload).await?;

    Ok(SendCompleted)
}
