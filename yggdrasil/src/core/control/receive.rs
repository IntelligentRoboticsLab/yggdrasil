use super::ControlData;
use crate::prelude::*;
use miette::{miette, IntoDiagnostic};
use serde::{Deserialize, Serialize};
use std::io;
use std::sync::Arc;
use tokio::net::TcpStream;

pub struct ControlReceiveModule;

impl Module for ControlReceiveModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_task::<AsyncTask<Result<Option<ClientMsg>>>>()?
            .add_system(listen_for_messages))
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClientMsg(String);

async fn read_request(stream: Arc<TcpStream>) -> Result<Option<ClientMsg>> {
    // Store somewhere instead of instatiating
    let mut msg = [0; 1024];

    stream.readable().await.into_diagnostic()?;

    match stream.try_read(&mut msg) {
        Ok(0) => Ok(None),
        Ok(num_bytes) => {
            let client_request: ClientMsg =
                bincode::deserialize(&msg[..num_bytes]).into_diagnostic()?;
            Ok(Some(client_request))
        }
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Err(miette!("Could not read")),
        Err(_) => Err(miette!("Something went wrong with reading")),
    }
}

#[system]
fn listen_for_messages(
    control_data: &mut ControlData,
    read_request_task: &mut AsyncTask<Result<Option<ClientMsg>>>,
) -> Result<()> {
    let Some(stream) = control_data.stream.clone() else {
        return Ok(());
    };

    if let Some(Ok(client_request)) = read_request_task.poll() {
        // Connection has been broken
        if client_request.is_none() {
            control_data.stream = None;
            return Ok(());
        }

        println!("Recieved request: {client_request:?}");
    }

    let _ = read_request_task.try_spawn(read_request(stream));

    Ok(())
}
