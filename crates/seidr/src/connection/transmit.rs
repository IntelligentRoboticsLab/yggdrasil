use async_std::{io::WriteExt, net::TcpStream};
use futures::{channel::mpsc::UnboundedReceiver, io::WriteHalf, StreamExt};
use miette::IntoDiagnostic;

use yggdrasil::core::control::receive::ControlClientMessage;

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
        stream
            .write(&serialized_msg_size)
            .await
            .expect("Failed writing to the robot stream");

        stream
            .write_all(&serialized_msg)
            .await
            .expect("Failed writing the control message to the stream");

        tracing::debug!("Send message: {:#?}", message);
    }

    tracing::debug!("Stopping send messages loop")
}
