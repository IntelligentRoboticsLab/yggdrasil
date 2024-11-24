pub mod receive;
pub mod transmit;

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use bevy::{
    prelude::*,
    tasks::{block_on, IoTaskPool},
};
use futures::channel::mpsc::unbounded;
use re_control_comms::{
    debug_system::DebugEnabledSystems,
    app::ControlApp,
    protocol::{ViewerMessage, CONTROL_PORT},
};

use receive::{
    handle_notify_on_connection, handle_viewer_message, NotifyConnectionReceiver,
    ViewerMessageReceiver,
};
use transmit::debug_systems_on_connection;
use uuid::Uuid;

pub struct ControlPlugin;

impl Plugin for ControlPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugEnabledSystems>()
            .add_event::<ViewerConnectedEvent>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (handle_notify_on_connection, debug_systems_on_connection)
                    .run_if(resource_exists::<ViewerMessageReceiver>),
            )
            .add_systems(
                Update,
                handle_viewer_message.run_if(resource_exists::<ViewerMessageReceiver>),
            );
    }
}

fn setup(mut commands: Commands) {
    let socket_addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, CONTROL_PORT));

    let (tx_on_connection, rx_on_connection) = unbounded();
    let notify_connection_receiver = NotifyConnectionReceiver {
        rx: rx_on_connection,
    };

    let io = IoTaskPool::get();
    let mut handle = block_on(io.spawn(async move {
        let app = ControlApp::bind(socket_addr, tx_on_connection)
            .await
            .unwrap_or_else(|_| panic!("Failed to bind control app to {socket_addr:?}"));

        app.run()
    }));

    let (tx, rx) = unbounded::<ViewerMessage>();
    handle.add_handler(tx).unwrap();

    let viewer_message_receiver = ViewerMessageReceiver { rx };

    commands.insert_resource(viewer_message_receiver);
    commands.insert_resource(handle);
    commands.insert_resource(notify_connection_receiver);
}

#[derive(Event)]
pub struct ViewerConnectedEvent(Uuid);
