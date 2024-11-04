pub mod connect;
pub mod receive;
pub mod transmit;

use std::{
    net::{Ipv4Addr, SocketAddrV4},
    sync::Arc,
};

use async_std::net::TcpListener;
use bevy::{
    prelude::*,
    tasks::{block_on, IoTaskPool},
};
use miette::{IntoDiagnostic, Result};

use connect::{listen_for_connection, setup_new_connection, ControlDataStream};
use receive::{handle_message, ControlViewerMessage, ControlReceiver};
use tasks::conditions::task_finished;
use transmit::{
    send_current_state, ControlRobotMessage, ControlSender, TransmitDebugEnabledSystems,
};

use super::debug::debug_system::DebugEnabledSystems;

pub const CONTROL_PORT: u16 = 40001;

pub struct ControlPlugin;

impl Plugin for ControlPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TransmitDebugEnabledSystems>()
            .init_resource::<DebugEnabledSystems>()
            .add_systems(Startup, setup)
            .add_systems(
                Update,
                (listen_for_connection, setup_new_connection)
                    .chain()
                    .run_if(not(resource_exists::<ControlDataStream>
                        .and_then(task_finished::<ControlDataStream>))),
            )
            .add_systems(
                Update,
                handle_message.run_if(resource_exists::<ControlReceiver<ControlViewerMessage>>),
            )
            .add_systems(
                Update,
                send_current_state.run_if(resource_exists::<ControlSender<ControlRobotMessage>>),
            );
    }
}

#[derive(Resource, Clone)]
pub struct ControlListenSocket {
    socket: Arc<TcpListener>,
}

impl ControlListenSocket {
    async fn bind() -> Result<Self> {
        let socket_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, CONTROL_PORT);
        let socket = TcpListener::bind(socket_addr).await.into_diagnostic()?;

        let socket = Arc::new(socket);

        Ok(Self { socket })
    }
}

fn setup(mut commands: Commands) {
    let io = IoTaskPool::get();
    let control_listen_socket = block_on(io.spawn(ControlListenSocket::bind()))
        .expect("Failed to bind control listen socket");
    commands.insert_resource(control_listen_socket);
}
