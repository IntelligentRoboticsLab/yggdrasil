use super::ControlData;
use crate::prelude::*;
use miette::IntoDiagnostic;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

pub struct ControlConnectModule;

impl Module for ControlConnectModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_task::<AsyncTask<Result<(TcpStream, SocketAddr)>>>()?
            .add_system(listen_for_connection))
    }
}

async fn accept_connection(listener_socket: Arc<TcpListener>) -> Result<(TcpStream, SocketAddr)> {
    listener_socket.accept().await.into_diagnostic()
}

#[system]
fn listen_for_connection(
    control_data: &mut ControlData,
    accept_connections_task: &mut AsyncTask<Result<(TcpStream, SocketAddr)>>,
) -> Result<()> {
    if control_data.stream.is_some() {
        return Ok(());
    }

    if let Some(Ok((socket, _addr))) = accept_connections_task.poll() {
        control_data.stream = Some(Arc::new(socket));
        println!("Setup connection");
        return Ok(());
    }

    let _ =
        accept_connections_task.try_spawn(accept_connection(control_data.listener_socket.clone()));

    Ok(())
}
