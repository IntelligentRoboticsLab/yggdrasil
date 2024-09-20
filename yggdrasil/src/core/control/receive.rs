use super::ControlData;
use crate::prelude::*;
use miette::{miette, IntoDiagnostic};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io;
use std::sync::Arc;
use tokio::net::TcpStream;
use tyr::{InspectView, InspectableResource};

pub struct ControlReceiveModule;

impl Module for ControlReceiveModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_task::<AsyncTask<Result<Option<ClientRequest>>>>()?
            .add_task::<AsyncTask<Result<StateUpdateRequest>>>()?
            .add_task::<AsyncTask<Result<UpdateConfigFinished>>>()?
            .add_system(listen_for_messages)
            .add_system(poll_update_resource.after(listen_for_messages)))
    }
}

pub struct StateUpdateRequest;
pub struct UpdateConfigFinished;

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientRequest {
    RobotState,
    ResourceUpdate(String, String),
}

async fn read_request(stream: Arc<TcpStream>) -> Result<Option<ClientRequest>> {
    // Store somewhere instead of instatiating
    let mut msg = [0; 4096];

    stream.readable().await.into_diagnostic()?;

    match stream.try_read(&mut msg) {
        Ok(0) => Ok(None),
        Ok(num_bytes) => {
            let client_request: ClientRequest =
                bincode::deserialize(&msg[..num_bytes]).into_diagnostic()?;
            Ok(Some(client_request))
        }
        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => Err(miette!("Could not read")),
        Err(_) => Err(miette!("Something went wrong with reading")),
    }
}

async fn communicate_manual_state_update() -> Result<StateUpdateRequest> {
    Ok(StateUpdateRequest)
}

async fn update_resource(
    resource: InspectableResource,
    new_resource: String,
) -> Result<UpdateConfigFinished> {
    let mut writable_resource = resource.write().unwrap();
    let json: Value = serde_json::from_str(&new_resource).into_diagnostic()?;
    writable_resource.try_update_from_json(json);
    Ok(UpdateConfigFinished)
}

#[system]
pub fn listen_for_messages(
    control_data: &mut ControlData,
    read_request_task: &mut AsyncTask<Result<Option<ClientRequest>>>,
    communicate_manual_state_update_task: &mut AsyncTask<Result<StateUpdateRequest>>,
    update_resource_task: &mut AsyncTask<Result<UpdateConfigFinished>>,
    inspect_view: &InspectView,
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

        match client_request.unwrap() {
            ClientRequest::RobotState => {
                let _ = communicate_manual_state_update_task
                    .try_spawn(communicate_manual_state_update());
            }
            ClientRequest::ResourceUpdate(resource_name, mut new_config) => {
                new_config.retain(|c| !c.is_whitespace());
                tracing::info!("Request to update resource: {resource_name}, with: {new_config}");
                if let Some(resource) = inspect_view.by_name(&resource_name) {
                    let _ = update_resource_task
                        .try_spawn(update_resource(resource.clone(), new_config));
                }
            }
        }
    }
    // Spawn the read_request task again because the current is finished
    let _ = read_request_task.try_spawn(read_request(stream));

    Ok(())
}

#[system]
fn poll_update_resource(
    update_resource_task: &mut AsyncTask<Result<UpdateConfigFinished>>,
) -> Result<()> {
    let Some(task_finished) = update_resource_task.poll() else {
        return Ok(());
    };

    if let Err(e) = task_finished {
        tracing::error!("Resource failed to update: {e}");
        return Ok(());
    }

    Ok(())
}
