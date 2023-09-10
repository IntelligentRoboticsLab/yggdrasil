pub mod message;
pub mod stream;

use std::net::SocketAddr;

use miette::{miette, Result};
use tyr::{
    prelude::*,
    tasks::{AsyncDispatcher, Error, Task, TaskSet},
};

use stream::{Connections, Listener};

use self::{
    message::{Message, Payload},
    stream::{Receiver, Sender},
};

pub const ADDR: &str = "0.0.0.0:1984";

pub struct WebSocketModule;

impl Module for WebSocketModule {
    fn initialize(self, app: App) -> Result<App> {
        use crate::nao;

        Ok(app
            .add_resource(Resource::<Connections>::default())?
            .add_resource(Resource::<Task<Result<(Sender, Receiver)>>>::default())?
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
    let socket = storage
        .map_resource_ref(|ad: &AsyncDispatcher| ad.handle().block_on(Listener::bind(ADDR)))?;

    tracing::info!("WebSocket listening on {ADDR}");

    storage.add_resource(Resource::new(socket))?;

    Ok(())
}

#[system]
fn accept_sockets(
    ad: &AsyncDispatcher,
    accept_task: &mut Task<Result<(Sender, Receiver)>>,
    recv_tasks: &mut TaskSet<Result<RecvCompletion>>,
    connections: &mut Connections,
    socket: &Listener,
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
                ad.dispatch_set(&mut recv_tasks, recv_message(receiver));

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
            // NOTE: this way we receive only one message per connection every
            // LoLA cycle. Shouldn't be an issue but something to keep in mind
            RecvCompletion::Message { rx, msg } => {
                // Try to receive another message 😎
                ad.dispatch_set(&mut recv_tasks, recv_message(rx));

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
    Message { rx: Receiver, msg: Message },
}

async fn recv_message(mut rx: Receiver) -> Result<RecvCompletion> {
    // Receive a single message in stream
    let Some(msg) = rx.recv_next().await? else {
        // No more messages, the connection is likely closed
        return Ok(RecvCompletion::ConnectionClosed(rx.address));
    };

    Ok(RecvCompletion::Message { rx, msg })
}

struct SendCompletion;

async fn send_message(conn: Sender, payload: Payload) -> Result<SendCompletion> {
    conn.send(payload).await?;

    Ok(SendCompletion)
}
