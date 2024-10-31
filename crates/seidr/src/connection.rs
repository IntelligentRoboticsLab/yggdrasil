use std::sync::{Arc, Mutex};

use async_std::{
    io::{ReadExt, WriteExt},
    net::{TcpStream, ToSocketAddrs},
};
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender}, io::{ReadHalf, WriteHalf}, stream::FusedStream, AsyncReadExt, StreamExt
};
use miette::{IntoDiagnostic, Result};
use tokio::time::Duration;

use yggdrasil::core::control::{
    receive::{ControlClientMessage, ControlReceiver},
    transmit::ControlHostMessage,
};

use crate::seidr::SeidrStates;

pub struct RobotConnection {
    pub reader: ReadHalf<TcpStream>,
    pub writer: WriteHalf<TcpStream>,
}

impl RobotConnection {
    async fn try_from_ip<A>(addr: A) -> Result<Self>
    where
        A: ToSocketAddrs,
    {
        let stream = TcpStream::connect(addr).await.into_diagnostic()?;
        let (reader, writer) = stream.split();
        let connection = RobotConnection { reader, writer };
        Ok(connection)
    }

    pub async fn try_connect<A>(addr: A, connection_attempts: i32) -> Result<Self>
    where
        A: ToSocketAddrs + Clone,
    {
        let mut attempt = 0;
        let connection = loop {
            match RobotConnection::try_from_ip(addr.clone()).await {
                Ok(conn) => break conn,
                Err(err) => {
                    tracing::info!(
                        "[{}/{}] Failed to connect: {}. Retrying...",
                        attempt,
                        connection_attempts,
                        err
                    );

                    if attempt >= connection_attempts {
                        tracing::error!("Max connections attempts reached");
                        std::process::exit(1);
                    }

                    attempt += 1;

                    // Wait before retrying
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            };
        };
        Ok(connection)
    }
}

pub async fn send_messages(
    mut stream: WriteHalf<TcpStream>,
    mut receiver: UnboundedReceiver<ControlClientMessage>,
) {
    while let Some(message) = receiver.next().await {
        let serialized_msg = bincode::serialize(&message)
            .into_diagnostic()
            .expect("Was not able to serialize a ControlHostMessage");

        let msg_size = serialized_msg.len();
        let serialized_msg_size = bincode::serialize(&msg_size).into_diagnostic().unwrap();
        stream.write(&serialized_msg_size).await.expect("Failed writing to the robot stream");

        stream
            .write_all(&serialized_msg)
            .await
            .expect("Failed writing the control message to the stream");

        tracing::info!("Send message: {:#?}", message);
    }

    tracing::info!("Stopping send messages loop")
}

pub async fn receive_messages(
    mut stream: ReadHalf<TcpStream>,
    sender: UnboundedSender<ControlHostMessage>,
) {
    let mut size_buffer = [0; std::mem::size_of::<usize>()];
    loop {
        if sender.is_closed() {
            tracing::warn!("Receiving message channel is closed");  
            break;
        }

        // The first message that will be read should be the size of the
        // next incoming message
        let num_bytes = ReadExt::read(&mut stream, &mut size_buffer).await.unwrap();

        if num_bytes == 0 {
            sender.unbounded_send(ControlHostMessage::CloseStream);
            break;
        }

        let msg_size = bincode::deserialize::<usize>(&size_buffer).unwrap();

        // Read the actual robot message from the stream
        let mut buffer = vec![0; msg_size];
        ReadExt::read_exact(&mut stream, &mut buffer).await.unwrap();
        // Decode message
        let message: ControlHostMessage = bincode::deserialize(&buffer).unwrap();

        // transmit decoded message to the channel
        sender.unbounded_send(message);
    }
}

pub enum HandleMessageStatus {
    Stopped,
    Continue,
}

pub fn handle_message(
    receiver: &mut ControlReceiver<ControlHostMessage>,
    states: &mut SeidrStates,
) -> HandleMessageStatus {
    while let Some(message) = receiver.try_recv() {
        match message {
            ControlHostMessage::CloseStream => {
                tracing::warn!("Connection is closed");
                return HandleMessageStatus::Stopped;
            }
            ControlHostMessage::Resources(new_resources) => {
                tracing::info!("Resource message: {:#?}", new_resources);
                let last_update = &states.last_resource_update;

                // if let Err(err) = states
                //     .robot_resources
                //     .update_resources(new_resources, &mut states.focused_resources)
                // {
                //     tracing::error!("Failed to update resources: {}", err);
                // }
                states
                    .robot_resources
                    .update_resources(new_resources, &mut states.focused_resources)
                    .unwrap();
            }
            ControlHostMessage::DebugEnabledResources(debug_enabled_resources) => {
                tracing::info!("Debug enabled resources init:\n{:#?}", debug_enabled_resources.resources);
                states.debug_enabled_resources_view = debug_enabled_resources.into();
            }
            _ => tracing::info!("Got a message to handle"),
        }
    }

    HandleMessageStatus::Continue
}
