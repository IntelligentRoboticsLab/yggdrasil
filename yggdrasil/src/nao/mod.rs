use std::{
    env::{self, VarError},
    time::Duration,
};

use miette::{IntoDiagnostic, Result};
use nidhogg::{
    backend::{ConnectWithRetry, LolaBackend, ReadHardwareInfo},
    NaoBackend, NaoControlMessage, NaoState,
};
use tracing::info;
use tyr::prelude::*;

pub struct RobotInfo {
    pub name: String,
    pub id: u32,
    pub head_id: String,
    pub body_id: String,
}

impl RobotInfo {
    fn new(head_id: String, body_id: String) -> Result<Self> {
        let name = env::var("ROBOT_NAME").into_diagnostic()?;
        let id = str::parse(&env::var("ROBOT_ID").into_diagnostic()?).into_diagnostic()?;

        Ok(Self {
            name,
            id,
            head_id,
            body_id,
        })
    }
}

/// This module provides the following resources to the application:
/// - [`LolaBackend`]
/// - [`NaoState`]
/// - [`NaoControlMessage`]
/// - [`RobotInfo`]
pub struct NaoModule;

impl Module for NaoModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .add_startup_system(initialize_nao)?
            .init_resource::<NaoControlMessage>()?
            .add_system(write_hardware_info))
    }
}

fn initialize_nao(storage: &mut Storage) -> Result<()> {
    let mut nao = LolaBackend::connect_with_retry(10, Duration::from_millis(500))?;
    let state = nao.read_nao_state()?;

    let info = nao.read_hardware_info()?;
    info!(
        "Launched yggdrasil on nao with head_id: {}, body_id: {}",
        info.head_id, info.body_id
    );

    storage.add_resource(Resource::new(nao))?;
    storage.add_resource(Resource::new(state))?;
    storage.add_resource(Resource::new(RobotInfo::new(info.head_id, info.body_id)))?;

    Ok(())
}

#[system]
pub fn write_hardware_info(
    nao: &mut LolaBackend,
    robot_state: &mut NaoState,
    update: &NaoControlMessage,
) -> Result<()> {
    *robot_state = nao.read_nao_state()?;
    nao.send_control_msg(update.clone())?;
    Ok(())
}
