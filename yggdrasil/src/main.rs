mod kinematics;

use std::time::Duration;

use color_eyre::Result;
use kinematics::bones;
use nalgebra::{Isometry3, Vector3};
use nidhogg::Nao;
use tracing::{info, warn};
use tyr::{data::*, scheduler::*, system::*};

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
async fn run_ik(position: &mut JointArray<f32>) {
    const TORSO_SHIFT_OFFSET: f32 = 0.013;
    const WALK_HIP_HEIGHT: f32 = 0.185;

    let left_foot_lift = 0.05;
    let right_foot_lift = 0.0;

    let left_foot_to_torso = calculate_foot_to_robot(
        Side::Left,
        FootOffsets {
            forward: 0.05,
            left: 0.0,
        },
        0f32,
        left_foot_lift,
        TORSO_SHIFT_OFFSET,
        WALK_HIP_HEIGHT,
    );
    let right_foot_to_torso = calculate_foot_to_robot(
        Side::Right,
        FootOffsets::zero(),
        0f32,
        right_foot_lift,
        TORSO_SHIFT_OFFSET,
        WALK_HIP_HEIGHT,
    );
    let (possible, left_leg, right_leg) =
        kinematics::inverse::leg_angles(left_foot_to_torso, right_foot_to_torso);

    println!("IK results: {possible}, left: {left_leg:?}, right: {right_leg:?}");

    if !possible {
        warn!("Impossible move!");
    }

    *position = JointArray::builder()
        .left_leg_joints(left_leg)
        .right_leg_joints(right_leg)
        .build();
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
    sched.add(run_ik());

    sched.run().await;

    Ok(())
}
