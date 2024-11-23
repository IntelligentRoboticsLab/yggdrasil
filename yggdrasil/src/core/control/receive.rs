use async_std::{io::ReadExt, net::TcpStream};
use bevy::prelude::*;
use control::connection::{
    app::ControlAppHandle,
    protocol::{RobotMessage, ViewerMessage},
};
use futures::{
    channel::mpsc::{self, UnboundedReceiver, UnboundedSender},
    io::ReadHalf,
};
use serde::{Deserialize, Serialize};

use super::ViewerConnectedEvent;

#[derive(Resource)]
pub struct ViewerMessageReceiver {
    pub rx: UnboundedReceiver<ViewerMessage>,
}

impl ViewerMessageReceiver {
    pub fn try_recv(&mut self) -> Option<ViewerMessage> {
        self.rx
            .try_next()
            .transpose()
            .expect("Control message receive channel closed")
            .ok()
    }
}

pub fn handle_viewer_message(
    mut message_receiver: ResMut<ViewerMessageReceiver>,
    mut ev_viewer_connected: EventWriter<ViewerConnectedEvent>,
) {
    while let Some(message) = message_receiver.try_recv() {
        match message {
            ViewerMessage::Disconnect => {
                tracing::info!("Viewer disconnected")
            }
            ViewerMessage::ViewerId(viewer_id) => {
                tracing::info!("New client connected: {viewer_id}");
                ev_viewer_connected.send(ViewerConnectedEvent(viewer_id));
            }
            _ => tracing::warn!("Unhandled message"),
        }
    }
}

// #[derive(Resource)]
// pub struct ControlReceiver<T> {
//     pub rx: mpsc::UnboundedReceiver<T>,
// }

// impl<T> ControlReceiver<T> {
//     pub fn try_recv(&mut self) -> Option<T> {
//         self.rx
//             .try_next()
//             .transpose()
//             .expect("Control message receive channel closed")
//             .ok()
//     }
// }

#[derive(Resource, Serialize, Deserialize, Debug)]
pub enum ControlViewerMessage {
    CloseStream,
    UpdateResource(String, String),
    SendResourcesNow,
    UpdateEnabledDebugSystem(String, bool),
}

pub async fn receive_messages(
    mut stream: ReadHalf<TcpStream>,
    sender: UnboundedSender<ControlViewerMessage>,
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
                .unbounded_send(ControlViewerMessage::CloseStream)
                .unwrap();
            continue;
        }

        let msg_size = bincode::deserialize::<usize>(&size_buffer).unwrap();
        let mut buffer = vec![0; msg_size];
        stream.read_exact(&mut buffer).await.unwrap();

        let msg = bincode::deserialize::<ControlViewerMessage>(&buffer).unwrap();

        sender.unbounded_send(msg).unwrap();
    }
}

// pub fn handle_message(
//     mut commands: Commands,
//     mut receiver: ResMut<ControlReceiver<ControlViewerMessage>>,
//     mut debug_enabled_systems: ResMut<DebugEnabledSystems>,
// ) {
//     while let Some(message) = receiver.try_recv() {
//         match message {
//             ControlViewerMessage::CloseStream => {
//                 commands.remove_resource::<ControlReceiver<ControlViewerMessage>>();
//                 commands.remove_resource::<ControlSender<ControlRobotMessage>>();
//                 commands.remove_resource::<ControlDataStream>();
//             }
//             ControlViewerMessage::UpdateEnabledDebugSystem(system_name, enabled) => {
//                 debug_enabled_systems.set_system(system_name.clone(), enabled);
//                 tracing::debug!("Set resource `{}` to `{}`", system_name, enabled);
//             }
//             _ => {
//                 tracing::warn!("Received a message which is not handled: {:?}", message);
//             }
//         }
//     }
// }

// #[derive(Serialize, Deserialize, Debug)]
// pub enum ClientRequest {
//     RobotState,
//     ResourceUpdate(String, String),
// }
