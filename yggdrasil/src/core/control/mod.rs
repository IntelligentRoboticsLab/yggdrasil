pub mod receive;
pub mod transmit;

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use bevy::prelude::*;
use futures::channel::mpsc::unbounded;
use yggdrasil_rerun_comms::{
    app::ControlApp,
    protocol::{CONTROL_PORT, ViewerMessage},
};

use receive::{
    ControlReceivePlugin, NotifyConnectionReceiver, ViewerMessageReceiver,
    handle_notify_on_connection, handle_viewer_control_message,
};
use transmit::ControlTransmitPlugin;

use super::debug::{RerunStream, init_rerun};

pub struct ControlPlugin;

impl Plugin for ControlPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup.after(init_rerun))
            .add_plugins((ControlReceivePlugin, ControlTransmitPlugin));
    }
}

fn setup(mut commands: Commands, rerun_stream: Res<RerunStream>) {
    if !rerun_stream.is_enabled() {
        return;
    }

    let socket_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, CONTROL_PORT));

    // New connections are registered and handled in the `ControlApp`. A
    // channel is used to pass the message to a Bevy system for additional
    // handling
    let (tx_on_connection, rx_on_connection) = unbounded();
    let notify_connection_receiver = NotifyConnectionReceiver {
        rx: rx_on_connection,
    };

    // Starts the control app and opens a listener for a yggdrasil_rerun viewer connection
    let app = ControlApp::bind(socket_addr, tx_on_connection)
        .unwrap_or_else(|_| panic!("Failed to bind control app to {socket_addr:?}"));
    let mut handle = app.run();

    // A channel is used to pass a `ViewerMessage` from the `ControlApp` to
    // `Resource` for use in a Bevy system
    let (tx, rx) = unbounded::<ViewerMessage>();
    handle.add_handler(tx).unwrap();
    let viewer_message_receiver = ViewerMessageReceiver { rx };

    commands.insert_resource(viewer_message_receiver);
    commands.insert_resource(handle);
    commands.insert_resource(notify_connection_receiver);
}
