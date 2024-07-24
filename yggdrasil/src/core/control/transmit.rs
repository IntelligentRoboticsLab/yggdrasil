use super::receive::{listen_for_messages, StateUpdateRequest};
use super::ControlData;
use crate::nao::Cycle;
use crate::prelude::*;
use miette::{miette, IntoDiagnostic};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use tokio::net::TcpStream;
use tyr::InspectView;

// The number of cycles between each send state to rerun
const SEND_STATE_PER_CYCLE: usize = 100;

pub struct ControlTransmitModule;

impl Module for ControlTransmitModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_task::<AsyncTask<Result<SendStateFinished>>>()?
            .init_resource::<SendStateCounter>()?
            .add_system(send_state_current_state.after(listen_for_messages)))
    }
}

pub struct SendStateFinished;

#[derive(Default)]
pub struct SendStateCounter(pub usize);

#[derive(Serialize, Deserialize, Debug)]
pub struct RobotStateMsg(HashMap<String, String>);

impl From<&InspectView> for RobotStateMsg {
    fn from(inspect_view: &InspectView) -> Self {
        let mut resource_map = HashMap::new();
        let resources = inspect_view.resources();
        for resource in resources {
            let locked_resource = resource.read().unwrap();
            let resource_name = locked_resource.name().to_string();
            let resource_json = locked_resource.to_json().to_string();
            resource_map.insert(resource_name, resource_json);
        }
        RobotStateMsg(resource_map)
    }
}

async fn send_state(stream: Arc<TcpStream>, state: RobotStateMsg) -> Result<SendStateFinished> {
    let msg = bincode::serialize(&state).into_diagnostic()?;

    send_message(stream, msg).await?;

    Ok(SendStateFinished)
}

async fn send_message(stream: Arc<TcpStream>, msg: Vec<u8>) -> Result<()> {
    stream.writable().await.into_diagnostic()?;

    match stream.try_write(&msg) {
        Ok(_num_bytes) => (),
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
            return Err(miette!("Could not read"))
        }
        Err(_) => return Err(miette!("Something went wrong with reading")),
    }
    Ok(())
}

#[system]
fn send_state_current_state(
    control_data: &mut ControlData,
    send_state_task: &mut AsyncTask<Result<SendStateFinished>>,
    communicate_manual_state_update_task: &mut AsyncTask<Result<StateUpdateRequest>>,
    inspect_view: &InspectView,
    current_cycle: &Cycle,
) -> Result<()> {
    // No need for the system to execute further if the stream does not exist
    let Some(stream) = control_data.stream.clone() else {
        return Ok(());
    };

    // Send current robot state immediately if requested
    if let Some(Ok(_)) = communicate_manual_state_update_task.poll() {
        let msg = RobotStateMsg::from(inspect_view);
        let Some(_) = send_state_task.poll() else {
            return Ok(());
        };
        let _ = send_state_task.try_spawn(send_state(stream, msg));
        return Ok(());
    };

    // Poll the send_state_task only every X cycles
    if current_cycle.0 % SEND_STATE_PER_CYCLE == 0 {
        // Collect the robot state and create the message
        let msg = RobotStateMsg::from(inspect_view);

        let Some(_) = send_state_task.poll() else {
            // When the task is not finished and not active (
            // this scenario is when there is a connection and the
            // task was not spawned before
            if !send_state_task.active() {
                let _ = send_state_task.try_spawn(send_state(stream, msg));
            }
            return Ok(());
        };
        // Spawn a new stask to send the current state because the old
        // task is finished
        let _ = send_state_task.try_spawn(send_state(stream, msg));
    }

    Ok(())
}
