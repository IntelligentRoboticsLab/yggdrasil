use async_std::{io::ReadExt, net::TcpStream};
use bevy::prelude::*;
use futures::{
    channel::mpsc::{self, UnboundedSender},
    io::ReadHalf,
};
use serde::{Deserialize, Serialize};

use crate::core::control::connect::ControlDataStream;

use super::transmit::ControlSender;

#[derive(Resource)]
pub struct ControlReceiver {
    pub rx: mpsc::UnboundedReceiver<ControlClientMessage>,
}

impl ControlReceiver {
    pub fn try_recv(&mut self) -> Option<ControlClientMessage> {
        self.rx
            .try_next()
            .transpose()
            .expect("Control message receive channel closed")
            .ok()
    }
}

#[derive(Resource, Serialize, Deserialize)]
pub enum ControlClientMessage {
    CloseStream,
    UpdateResource(String, String),
    SendResourcesNow,
}

pub async fn receive_messages(
    mut stream: ReadHalf<TcpStream>,
    sender: UnboundedSender<ControlClientMessage>,
) {
    loop {
        if sender.is_closed() {
            info!("Breaking up receive message loop");
            break;
        }
        let mut buffer = Vec::new();
        let Ok(num_bytes) = stream.read_to_end(&mut buffer).await else {
            todo!("Do something if reading of stream goes wrong")
        };

        if num_bytes == 0 {
            sender.unbounded_send(ControlClientMessage::CloseStream);
            continue;
        }

        sender.unbounded_send(ControlClientMessage::SendResourcesNow);
    }
}

pub fn handle_message(mut commands: Commands, mut receiver: ResMut<ControlReceiver>) {
    while let Some(message) = receiver.try_recv() {
        match message {
            ControlClientMessage::CloseStream => {
                commands.remove_resource::<ControlReceiver>();
                commands.remove_resource::<ControlSender>();
                commands.remove_resource::<ControlDataStream>();
            }
            _ => {
                info!("Got a message to handle");
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientRequest {
    RobotState,
    ResourceUpdate(String, String),
}
