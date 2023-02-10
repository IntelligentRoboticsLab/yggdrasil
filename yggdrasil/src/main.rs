mod kinematics;
mod types;

use std::time::Duration;

use color_eyre::Result;
use kinematics::bones;
use nalgebra::{Isometry3, Vector3};
use nidhogg::Nao;
use tyr::{data::*, scheduler::*, system::*};

use nidhogg::{types::JointArray, State, Update};

#[derive(Data)]
struct NaoState {
    nao: Nao,
    state: State,
    stiffness: JointArray<f32>,
    position: JointArray<f32>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct FootOffsets {
    pub forward: f32,
    pub left: f32,
}

impl FootOffsets {
    pub fn zero() -> Self {
        Self {
            forward: 0.0,
            left: 0.0,
        }
    }
}

pub enum Side {
    Left,
    Right,
}

pub fn calculate_foot_to_robot(
    side: Side,
    foot: FootOffsets,
    turn_left_right: f32,
    foot_lift: f32,
    torso_offset: f32,
    walk_hip_height: f32,
) -> Isometry3<f32> {
    let hip_to_robot = match side {
        Side::Left => Isometry3::from(bones::ROBOT_TO_LEFT_PELVIS),
        Side::Right => Isometry3::from(bones::ROBOT_TO_RIGHT_PELVIS),
    };
    let foot_rotation = match side {
        Side::Left => turn_left_right,
        Side::Right => -turn_left_right,
    };
    hip_to_robot
        * Isometry3::translation(
            foot.forward - torso_offset,
            foot.left,
            -walk_hip_height + foot_lift,
        )
        * Isometry3::rotation(Vector3::z() * foot_rotation)
}

#[system(NaoState)]
async fn run_ik(state: &State, stiffness: &mut JointArray<f32>, position: &mut JointArray<f32>) {
    let left_foot_to_torso = calculate_foot_to_robot(
        Side::Left,
        FootOffsets {
            forward: 3.0,
            left: 0.0,
        },
        0f32,
        1f32,
        1f32,
        1f32,
    );
    let right_foot_to_torso =
        calculate_foot_to_robot(Side::Right, FootOffsets::zero(), 0f32, 0f32, 0f32, 0f32);
    let (possible, left_leg, right_leg) =
        kinematics::inverse::leg_angles(left_foot_to_torso, right_foot_to_torso);

    println!("IK results: {possible}, left: {left_leg:?}, right: {right_leg:?}");

    stiffness.left_hip_pitch = 1.0;
    stiffness.left_hip_roll = 1.0;
    stiffness.left_hip_yaw_pitch = 1.0;
    stiffness.left_knee_pitch = 1.0;
    stiffness.left_ankle_pitch = 1.0;
    stiffness.left_ankle_roll = 1.0;

    stiffness.right_hip_pitch = 1.0;
    stiffness.right_hip_roll = 1.0;
    stiffness.right_knee_pitch = 1.0;
    stiffness.right_ankle_pitch = 1.0;
    stiffness.right_ankle_roll = 1.0;

    stiffness.left_hip_pitch = left_leg.hip_pitch;
    stiffness.left_hip_roll = left_leg.hip_roll;
    stiffness.left_hip_yaw_pitch = left_leg.hip_yaw_pitch;
    stiffness.left_knee_pitch = left_leg.knee_pitch;
    stiffness.left_ankle_pitch = left_leg.ankle_pitch;
    stiffness.left_ankle_roll = left_leg.ankle_roll;

    position.right_hip_pitch = right_leg.hip_pitch;
    position.right_hip_roll = right_leg.hip_roll;
    position.right_knee_pitch = right_leg.knee_pitch;
    position.right_ankle_pitch = right_leg.ankle_pitch;
    // position.right_ankle_roll = right_leg.ankle_roll;
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
    sched.add(run_ik());

    sched.run().await;

    Ok(())
}
