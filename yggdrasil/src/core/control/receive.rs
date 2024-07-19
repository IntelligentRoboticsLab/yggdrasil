use super::ControlData;
use crate::prelude::*;
use miette::{miette, IntoDiagnostic};
use serde::{Deserialize, Serialize};
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};

pub struct ControlReceiveModule;

impl Module for ControlReceiveModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_task::<AsyncTask<Result<ClientRequest>>>()?
            .add_system(listen_for_messages))
    }
}

#[derive(Serialize, Deserialize)]
pub struct ClientRequest;

async fn read_request(stream: Arc<TcpStream>) -> Result<ClientRequest> {
    // Store somewhere instead of instatiating
    let mut msg = [0; 1024];

    stream.readable().await.into_diagnostic()?;

    match stream.try_read(&mut msg) {
        Ok(num_bytes) => {
            // deserialize and return result
            // if num_bytes == 0
            // - closed
            // - msg has lenght 0
            // msg.truncate(n);
        }
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
            return Err(miette!("Could not read"));
        }
        Err(_) => {
            return Err(miette!("Something went wrong with reading"));
        }
    }

    Ok(ClientRequest)
}

#[system]
fn listen_for_messages(
    control_data: &mut ControlData,
    read_request_task: &mut AsyncTask<Result<ClientRequest>>,
) -> Result<()> {
    let Some(stream) = control_data.stream.clone() else {
        return Ok(());
    };

    if let Some(Ok(client_request)) = read_request_task.poll() {
        println!("Recieved request!");
    }

    let _ = read_request_task.try_spawn(read_request(stream));

    Ok(())
}
