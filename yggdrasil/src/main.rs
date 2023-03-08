mod kinematics;
mod movements;

use movements::{
	NaoState,
	NaoStateAccess,
	run_ik_new,
};

use std::time::Duration;

use color_eyre::Result;
// use kinematics::bones;
// use nalgebra::{Isometry3, Vector3};
use nidhogg::Nao;
use tracing::{
	info,
	// warn,
};
use tyr::{
	// data::*,
	scheduler::*,
	system::*};

use nidhogg::{types::JointArray, State, Update};

trait FillExt<T> {
    fn fill(self, value: T) -> Self;
}

impl FillExt<f32> for JointArray<f32> {
    fn fill(self, value: f32) -> Self {
        JointArray {
            head_yaw: value,
            head_pitch: value,
            left_shoulder_pitch: value,
            left_shoulder_roll: value,
            left_elbow_yaw: value,
            left_elbow_roll: value,
            left_wrist_yaw: value,
            left_hip_yaw_pitch: value,
            left_hip_roll: value,
            left_hip_pitch: value,
            left_knee_pitch: value,
            left_ankle_pitch: value,
            left_ankle_roll: value,
            right_hip_roll: value,
            right_hip_pitch: value,
            right_knee_pitch: value,
            right_ankle_pitch: value,
            right_ankle_roll: value,
            right_shoulder_pitch: value,
            right_shoulder_roll: value,
            right_elbow_yaw: value,
            right_elbow_roll: value,
            right_wrist_yaw: value,
            left_hand: value,
            right_hand: value,
        }
    }
}

#[system(NaoState)]
async fn read_write_data(
    nao: &mut Nao,
    state: &mut State,
    stiffness: &JointArray<f32>,
    position: &JointArray<f32>,
) {
    *state = nao.read_state().unwrap();
    info!("Read here!");

    nao.write_update(
        Update::builder()
            .stiffness(stiffness.clone())
            .position(position.clone())
            .build(),
    )
    .unwrap();

    info!("Wrote here!");
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let mut nao = Nao::connect_retry(10, Duration::from_secs(1))?;
    let state = nao.read_state()?;
    let hw = nao.read_hardware_info()?;

    println!("{:?}", hw);

    let mut sched = Scheduler::new(NaoState {
        nao,
        state,
        stiffness: JointArray::default().fill(0.3),
        position: JointArray::default(),
    });

    sched.add(read_write_data());

    sched.add(run_ik_new());

    sched.run().await;

    Ok(())
}
