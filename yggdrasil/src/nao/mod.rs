use std::time::Duration;

use miette::Result;
use nidhogg::{
    backend::{ConnectWithRetry, LolaBackend, ReadHardwareInfo},
    NaoBackend, NaoControlMessage, NaoState,
};
use tracing::info;
use tyr::prelude::*;

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
