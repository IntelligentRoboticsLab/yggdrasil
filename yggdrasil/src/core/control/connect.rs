use bevy::prelude::*;
use control::connection::{app::ControlAppHandle, protocol::{RobotMessage, ViewerMessage}};

use super::transmit::TransmitDebugEnabledSystems;


// #[derive(Resource)]
// pub struct ControlDataStream {
//     pub stream: TcpStream,
// }

// #[derive(Resource)]
// pub struct ControlWriteStream {
//     pub stream: WriteHalf<TcpStream>,
// }

// pub fn listen_for_connection(mut commands: Commands, listener_socket: Res<ControlListenSocket>) {
//     let socket = listener_socket.socket.clone();

//     commands.prepare_task(TaskPool::Io).to_resource().spawn({
//         async move {
//             let Ok((stream, _addr)) = socket.accept().await else {
//                 return None;
//             };

//             let control_stream = ControlDataStream { stream };

//             Some(control_stream)
//         }
//     });
// }

// pub fn on_control_connection(
//     mut commands: Commands,
//     control_handle: Res<ControlAppHandle>,
//     transmit_debug_system: Res<TransmitDebugEnabledSystems>,
// ) {
    
//     if control_handle.is_added() {
//         tracing::info!("Send debug systems");
//         commands.run_system(transmit_debug_system.system_id());
//     }
// }

// pub fn setup_new_connection(
//     mut commands: Commands,
//     control_stream: Option<Res<ControlDataStream>>,
//     transmit_debug_enabled_systems_system: Res<TransmitDebugEnabledSystems>,
// ) {
//     if let Some(control_stream) = control_stream {
//         if control_stream.is_added() {
//             let addr = control_stream
//                 .stream
//                 .peer_addr()
//                 .into_diagnostic()
//                 .expect("Could not read the control client address");

//             tracing::info!("Connected with a new control client: {}", addr.to_string());

//             let (reader, writer) = control_stream.stream.clone().split();

//             let io = IoTaskPool::get();
//             let (reader_tx, reader_rx) = mpsc::unbounded::<ControlViewerMessage>();
//             let (writer_tx, writer_rx) = mpsc::unbounded::<ControlRobotMessage>();

//             io.spawn(receive_messages(reader, reader_tx)).detach();
//             io.spawn(send_messages(writer, writer_rx)).detach();

//             commands.insert_resource(ControlReceiver { rx: reader_rx });
//             commands.insert_resource(ControlSender { tx: writer_tx });

//             // Sends the debug enabled systems to rerun for initialization
//             commands.run_system(transmit_debug_enabled_systems_system.system_id());
//         }
//     }
// }
