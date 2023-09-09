pub mod listener;
pub mod message;

use std::net::SocketAddr;

use bifrost::serialization::Encode;
use miette::{miette, IntoDiagnostic, Result};
use tyr::{
    prelude::*,
    tasks::{AsyncDispatcher, Error, Task, TaskSet},
};

use listener::{Connections, WebSocketListener};

use self::{
    listener::{WebSocketReceiver, WebSocketSender},
    message::{Message, Payload},
};

pub const ADDR: &str = "0.0.0.0:1984";

pub struct WebSocketModule;

impl Module for WebSocketModule {
    fn initialize(self, app: App) -> Result<App> {
        use crate::nao;

        Ok(app
            .add_resource(Resource::<Connections>::default())?
            .add_resource(
                Resource::<Task<Result<(WebSocketSender, WebSocketReceiver)>>>::default(),
            )?
            .add_resource(Resource::<TaskSet<Result<RecvCompletion>>>::default())?
            .add_resource(Resource::<TaskSet<Result<SendCompletion>>>::default())?
            .add_startup_system(init_server)?
            .add_system(accept_sockets)
            .add_system(recv_messages)
            .add_system(
                send_messages
                    .after(nao::write_hardware_info)
                    .after(recv_messages),
            ))
    }
}

fn init_server(storage: &mut Storage) -> Result<()> {
    let socket = storage.map_resource_ref(|ad: &AsyncDispatcher| {
        ad.handle().block_on(WebSocketListener::bind(ADDR))
    })?;

    tracing::info!("WebSocket listening on {ADDR}");

    storage.add_resource(Resource::new(socket))?;

    Ok(())
}

#[system]
fn accept_sockets(
    ad: &AsyncDispatcher,
    accept_task: &mut Task<Result<(WebSocketSender, WebSocketReceiver)>>,
    recv_tasks: &mut TaskSet<Result<RecvCompletion>>,
    connections: &mut Connections,
    socket: &WebSocketListener,
) -> Result<()> {
    match ad.try_dispatch(&mut accept_task, {
        let socket = socket.clone();
        async move { socket.accept().await }
    }) {
        Ok(()) => (),
        Err(Error::AlreadyDispatched) => {
            // Task is already dispatched so we poll it
            if let Some((sender, receiver)) = accept_task.poll().transpose()? {
                tracing::info!("Opened ws connection with {}", sender.address);

                // Start receiving messages
                ad.dispatch_set(&mut recv_tasks, recv_message(receiver, sender.address));

                connections.insert(sender);
            }
        }
    };

    Ok(())
}

#[system]
fn recv_messages(
    ad: &AsyncDispatcher,
    send_tasks: &mut TaskSet<Result<SendCompletion>>,
    recv_tasks: &mut TaskSet<Result<RecvCompletion>>,
    connections: &mut Connections,
) -> Result<()> {
    // Poll for new messages
    let msgs = recv_tasks.poll_all();

    for res in msgs {
        // Check if any connections got closed
        match res? {
            RecvCompletion::ConnectionClosed(address) => {
                connections.remove(address);
            }
            RecvCompletion::Message { rx, msg } => {
                // Receive more messages 😎
                ad.dispatch_set(&mut recv_tasks, recv_message(rx, msg.address));

                handle_message(msg, &ad, &mut send_tasks, &connections)?;
            }
        };
    }

    Ok(())
}

#[system]
fn send_messages(
    ad: &AsyncDispatcher,
    send_tasks: &mut TaskSet<Result<SendCompletion>>,
    connections: &Connections,
) -> Result<()> {
    for conn in connections.values() {
        ad.dispatch_set(
            &mut send_tasks,
            send_message(conn.clone(), Payload::text("Hello world!")),
        );
    }

    Ok(())
}

fn handle_message(
    msg: Message,
    ad: &AsyncDispatcher,
    send_tasks: &mut TaskSet<Result<SendCompletion>>,
    connections: &Connections,
) -> Result<()> {
    // handle message
    match msg.payload {
        Payload::Ping => {
            let conn = connections
                .get(msg.address)
                .ok_or_else(|| miette!("Connection with address `{}` not found", msg.address))?;

            // send back a pong
            ad.dispatch_set(send_tasks, send_message(conn.clone(), Payload::Pong));
        }
        Payload::Pong => (),
        Payload::Text(t) => tracing::debug!("Received text: `{t}`"),
    };

    Ok(())
}

enum RecvCompletion {
    ConnectionClosed(SocketAddr),
    Message { rx: WebSocketReceiver, msg: Message },
}

async fn recv_message(mut rx: WebSocketReceiver, address: SocketAddr) -> Result<RecvCompletion> {
    // Receive messages in stream
    let Some(payload) = rx.recv_next().await? else {
        // No more messages, the connection is likely closed
        return Ok(RecvCompletion::ConnectionClosed(address));
    };

    let msg = Message { address, payload };
    Ok(RecvCompletion::Message { rx, msg })
}

struct SendCompletion;

async fn send_message(conn: WebSocketSender, payload: Payload) -> Result<SendCompletion> {
    let mut buf = Vec::with_capacity(payload.encode_len());
    payload.encode(&mut buf).into_diagnostic()?;

    conn.send(buf).await?;

    Ok(SendCompletion)
}
