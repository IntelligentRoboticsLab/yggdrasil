use super::ControlData;
use crate::prelude::*;
use miette::{miette, IntoDiagnostic};
use serde::{Deserialize, Serialize};
use std::io;
use std::sync::Arc;
use tokio::net::TcpStream;

// TODO: send state every 100 cycles
pub struct ControlTransmitModule;

impl Module for ControlTransmitModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_task::<AsyncTask<Result<SendStateFinished>>()?
            .init_resource::<SendStateCounter>()
            .add_system(send_messages))
    }
}

pub struct SendStateFinished;

pub struct SendStateCounter(i32);

#[derive(Serialize, Deserialize, Debug)]
pub struct RobotMsg(String);

async fn send_state(stream: Arc<TcpStream>) -> Result<SendStateFinished> {
    // Store somewhere instead of instatiating
    // let mut msg = [0; 1024];
    //
    // stream.readable().await.into_diagnostic()?;
    //
    // match stream.try_read(&mut msg) {
    //     Ok(0) => Ok(None),
    //     Ok(num_bytes) => {
    //         let client_request: ClientRequest =
    //             bincode::deserialize(&msg[..num_bytes]).into_diagnostic()?;
    //         Ok(Some(client_request))
    //     }
    //     Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Err(miette!("Could not read")),
    //     Err(_) => Err(miette!("Something went wrong with reading")),
    // }

    Ok(SendStateFinished)
}

#[system]
fn send_messages(
    control_data: &mut ControlData,
    send_state_task: &mut AsyncTask<Result<SendStateFinished>>,
) -> Result<()> {
    let Some(stream) = control_data.stream.clone() else {
        return Ok(());
    };

    // Create msg
    let msg = None;

    let _ = send_state_task.try_spawn(send_state(stream));

    Ok(())
}
