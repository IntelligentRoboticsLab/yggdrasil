use std::time::Duration;

use miette::Result;
use nidhogg::{
    backend::{ConnectWithRetry, LolaBackend, ReadHardwareInfo},
    NaoBackend,
};
use tyr::prelude::*;

fn main() -> Result<()> {
    App::new()
        .init_resource::<nidhogg::NaoControlMessage>()?
        // startup systems that run once before the DAG is created
        // here we use it to add a `nidhog::Nao` and `nidhogg::State` to the storage so we dont need
        // to have Option<Nao> in our storage which would be annoying
        .add_startup_system(initialize_nao)?
        .add_system(update_lola)
        .run()
}

fn initialize_nao(storage: &mut Storage) -> Result<()> {
    let mut nao = LolaBackend::connect_with_retry(10, Duration::from_millis(500))?;
    let state = nao.read_nao_state()?;

    let info = nao.read_hardware_info()?;
    println!("{info:?}");

    storage.add_resource(Resource::new(nao))?;
    storage.add_resource(Resource::new(state))?;

    Ok(())
}

#[system]
fn update_lola(
    nao: &mut LolaBackend,
    robot_state: &mut nidhogg::NaoState,
    update: &nidhogg::NaoControlMessage,
) -> Result<()> {
    *robot_state = nao.read_nao_state()?;
    nao.send_control_msg(update.clone())?;

    Ok(())
}
