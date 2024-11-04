use async_std::{io::ReadExt, net::TcpStream};
use futures::{channel::mpsc::UnboundedSender, io::ReadHalf};

use yggdrasil::core::control::{receive::ControlReceiver, transmit::ControlRobotMessage};

use crate::control::ControlStates;

pub async fn receive_messages(
    mut stream: ReadHalf<TcpStream>,
    sender: UnboundedSender<ControlRobotMessage>,
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
            sender
                .unbounded_send(ControlRobotMessage::CloseStream)
                .unwrap();
            break;
        }

        let msg_size = bincode::deserialize::<usize>(&size_buffer).unwrap();

        // Read the actual robot message from the stream
        let mut buffer = vec![0; msg_size];
        ReadExt::read_exact(&mut stream, &mut buffer).await.unwrap();
        // Decode message
        let message: ControlRobotMessage = bincode::deserialize(&buffer).unwrap();

        // transmit decoded message to the channel
        sender.unbounded_send(message).unwrap();
    }
}

pub enum HandleMessageStatus {
    Stopped,
    Continue,
}

pub fn handle_message(
    receiver: &mut ControlReceiver<ControlRobotMessage>,
    states: &mut ControlStates,
) -> HandleMessageStatus {
    while let Some(message) = receiver.try_recv() {
        match message {
            ControlRobotMessage::CloseStream => {
                tracing::warn!("Connection is closed");
                return HandleMessageStatus::Stopped;
            }
            ControlRobotMessage::Resources(new_resources) => {
                tracing::debug!("Resource message: {:?}", new_resources);
                let _last_update = &states.last_resource_update;

                states
                    .robot_resources
                    .update_resources(new_resources, &mut states.focused_resources)
                    .unwrap();
            }
            ControlRobotMessage::DebugEnabledSystems(debug_enabled_systems) => {
                tracing::debug!(
                    "Debug enabled resources init:\n{:#?}",
                    debug_enabled_systems.systems
                );
                states.debug_enabled_systems_view = debug_enabled_systems.into();
            }
        }
    }

    HandleMessageStatus::Continue
}
