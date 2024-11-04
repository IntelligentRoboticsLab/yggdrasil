use async_std::{io::ReadExt, net::TcpStream};
use bevy::prelude::*;
use futures::{
    channel::mpsc::{self, UnboundedSender},
    io::ReadHalf,
};
use serde::{Deserialize, Serialize};

use crate::core::control::connect::ControlDataStream;

use super::{
    transmit::{ControlHostMessage, ControlSender},
    DebugEnabledSystems,
};

#[derive(Resource)]
pub struct ControlReceiver<T> {
    pub rx: mpsc::UnboundedReceiver<T>,
}

impl<T> ControlReceiver<T> {
    pub fn try_recv(&mut self) -> Option<T> {
        self.rx
            .try_next()
            .transpose()
            .expect("Control message receive channel closed")
            .ok()
    }
}

#[derive(Resource, Serialize, Deserialize, Debug)]
pub enum ControlClientMessage {
    CloseStream,
    UpdateResource(String, String),
    SendResourcesNow,
    UpdateEnabledDebugSystem(String, bool),
}

pub async fn receive_messages(
    mut stream: ReadHalf<TcpStream>,
    sender: UnboundedSender<ControlClientMessage>,
) {
    let mut size_buffer = [0; std::mem::size_of::<usize>()];
    loop {
        if sender.is_closed() {
            warn!("Breaking up receive message loop");
            break;
        }

        let num_bytes = stream.read(&mut size_buffer).await.unwrap();

        if num_bytes == 0 {
            sender
                .unbounded_send(ControlClientMessage::CloseStream)
                .unwrap();
            continue;
        }

        let msg_size = bincode::deserialize::<usize>(&size_buffer).unwrap();
        let mut buffer = vec![0; msg_size];
        stream.read_exact(&mut buffer).await.unwrap();

        let msg = bincode::deserialize::<ControlClientMessage>(&buffer).unwrap();

        sender.unbounded_send(msg).unwrap();
    }
}

pub fn handle_message(
    mut commands: Commands,
    mut receiver: ResMut<ControlReceiver<ControlClientMessage>>,
    mut debug_enabled_systems: ResMut<DebugEnabledSystems>,
) {
    while let Some(message) = receiver.try_recv() {
        match message {
            ControlClientMessage::CloseStream => {
                commands.remove_resource::<ControlReceiver<ControlClientMessage>>();
                commands.remove_resource::<ControlSender<ControlHostMessage>>();
                commands.remove_resource::<ControlDataStream>();
            }
            ControlClientMessage::UpdateEnabledDebugSystem(system_name, enabled) => {
                debug_enabled_systems.set_system(system_name.clone(), enabled);
                tracing::debug!("Set resource `{}` to `{}`", system_name, enabled);
            }
            _ => {
                tracing::warn!("Received a message which is not handled: {:?}", message);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientRequest {
    RobotState,
    ResourceUpdate(String, String),
}
