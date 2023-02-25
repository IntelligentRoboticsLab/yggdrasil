use std::time::Duration;

use color_eyre::Result;
use nidhogg::Nao;

use nidhogg::{types::JointArray, State, Update};
use tyr::data::Data;
use tyr::scheduler::Scheduler;
use tyr::system::system;

#[derive(Data)]
struct NaoState {
    nao: Nao,
    state: State,
    stiffness: JointArray<f32>,
    position: JointArray<f32>,
}

#[system(NaoState)]
async fn read_write_data(
    nao: &mut Nao,
    state: &mut State,
    stiffness: &JointArray<f32>,
    position: &JointArray<f32>,
) {
    *state = nao.read_state().unwrap();
    nao.write_update(
        Update::builder()
            .stiffness(stiffness.clone())
            .position(position.clone())
            .build(),
    )
    .unwrap();
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let mut nao = Nao::connect_retry(100, Duration::from_secs(1))?;
    let hw_info = nao.read_hardware_info()?;

    println!("{:?}", hw_info);

    let initial_state = nao.read_state()?;

    let mut sched = Scheduler::new(NaoState {
        nao,
        state: initial_state,
        stiffness: JointArray::default(),
        position: JointArray::default(),
    });

    sched.add(read_write_data());

    sched.run().await;

    Ok(())
}
