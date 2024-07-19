use crate::prelude::*;
use miette::{IntoDiagnostic, Result};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

pub mod connect;
pub mod receive;

const CONTROL_PORT: u16 = 40001;

pub struct ControlModule;

pub struct ControlData {
    listener_socket: Arc<TcpListener>,
    stream: Option<Arc<TcpStream>>,
}

impl ControlModule {
    async fn new_control_socket() -> Result<TcpListener> {
        let listener_socket =
            TcpListener::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, CONTROL_PORT))
                .await
                .into_diagnostic()?;

        Ok(listener_socket)
    }

    #[startup_system]
    fn add_resources(storage: &mut Storage, dispatcher: &AsyncDispatcher) -> Result<()> {
        let game_controller_socket = dispatcher.handle().block_on(Self::new_control_socket())?;

        storage.add_resource(Resource::new(ControlData {
            listener_socket: Arc::new(game_controller_socket),
            stream: None,
        }))?;

        Ok(())
    }
}

impl Module for ControlModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_startup_system(Self::add_resources)?
            .add_module(connect::ControlConnectModule)?
            .add_module(receive::ControlReceiveModule)
        // .add_module(transmit::GameControllerTransmitModule)
    }
}
