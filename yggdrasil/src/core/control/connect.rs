use async_std::net::TcpStream;
use bevy::{prelude::*, tasks::IoTaskPool};
use futures::{channel::mpsc, io::WriteHalf, AsyncReadExt};
use miette::IntoDiagnostic;
use tasks::{CommandsExt, TaskPool};

use super::{receive::ControlClientMessage, ControlListenSocket};
use crate::core::control::{
    receive::{receive_messages, ControlReceiver},
    transmit::{send_messages, ControlHostMessage, ControlSender},
};

#[derive(Resource)]
pub struct ControlDataStream {
    pub stream: TcpStream,
}

#[derive(Resource)]
pub struct ControlWriteStream {
    pub stream: WriteHalf<TcpStream>,
}

// impl ControlDataStream {
//     pub async fn recv(&mut self, buffer: &mut Vec<u8>) -> Result<usize> {
//         let mut reader = self.reader.lock().await;

//         let num_bytes = reader.read_to_end(buffer).await.into_diagnostic()?;
//         Ok(num_bytes)
//     }

//     pub async fn send(&mut self, buffer: &[u8]) -> io::Result<()> {
//         info!("In function send()");
//         let mut writer = self.writer.lock().await;
//         info!("Locked stream");
//         writer.write_all(buffer).await;
//         info!("Sending robot state");

//         Ok(())
//     }
// }

pub fn listen_for_connection(mut commands: Commands, listener_socket: Res<ControlListenSocket>) {
    let socket = listener_socket.socket.clone();

    commands.prepare_task(TaskPool::Io).to_resource().spawn({
        async move {
            let Ok((stream, _addr)) = socket.accept().await else {
                return None;
            };

            let control_stream = ControlDataStream { stream };

            Some(control_stream)
        }
    });
}

pub fn setup_new_connection(
    mut commands: Commands,
    control_stream: Option<Res<ControlDataStream>>,
) {
    if let Some(control_stream) = control_stream {
        if control_stream.is_added() {
            let addr = control_stream
                .stream
                .peer_addr()
                .into_diagnostic()
                .expect("Could not read the control client address");

            info!("Connected with a new control client: {}", addr.to_string());

            let (reader, writer) = control_stream.stream.clone().split();

            let io = IoTaskPool::get();
            let (reader_tx, reader_rx) = mpsc::unbounded::<ControlClientMessage>();
            let (writer_tx, writer_rx) = mpsc::unbounded::<ControlHostMessage>();

            io.spawn(receive_messages(reader, reader_tx)).detach();
            io.spawn(send_messages(writer, writer_rx)).detach();

            commands.insert_resource(ControlReceiver { rx: reader_rx });
            commands.insert_resource(ControlSender { tx: writer_tx });
        }
    }
}
