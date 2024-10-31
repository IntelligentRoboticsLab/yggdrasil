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
    DebugEnabledResources,
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
    UpdateEnabledDebugResource(String, bool),
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
            sender.unbounded_send(ControlClientMessage::CloseStream);
            continue;
        }

        let msg_size = bincode::deserialize::<usize>(&size_buffer).unwrap();
        let mut buffer = vec![0; msg_size];
        let _num_bytes = stream.read_exact(&mut buffer).await.unwrap();

        let msg = bincode::deserialize::<ControlClientMessage>(&buffer).unwrap();

        sender.unbounded_send(msg);
    }
}

pub fn handle_message(
    mut commands: Commands,
    mut receiver: ResMut<ControlReceiver<ControlClientMessage>>,
    mut debug_enabled_resources: ResMut<DebugEnabledResources>,
) {
    while let Some(message) = receiver.try_recv() {
        match message {
            ControlClientMessage::CloseStream => {
                commands.remove_resource::<ControlReceiver<ControlClientMessage>>();
                commands.remove_resource::<ControlSender<ControlHostMessage>>();
                commands.remove_resource::<ControlDataStream>();
            }
            ControlClientMessage::UpdateEnabledDebugResource(system_name, enabled) => {
                debug_enabled_resources.set_resource(system_name.clone(), enabled);
                tracing::info!("Set resource `{}` to `{}`", system_name, enabled)
            }
            _ => {
                info!("Got a message to handle: {:?}", message);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientRequest {
    RobotState,
    ResourceUpdate(String, String),
}
