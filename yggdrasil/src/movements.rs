use crate::kinematics;
// use std::time::Duration;

// use color_eyre::Result;
// use crate::kinematics::bones;
use nalgebra::{Isometry3, Vector3};
use nidhogg::Nao;
use tracing::{
    // info,
    warn
};
use tyr::{
    data::*,
    // scheduler::*,
    system::*};

use nidhogg::{
    types::JointArray,
    State,
    // Update
};

#[derive(Data)]
pub struct NaoState {
    pub nao: Nao,
    pub state: State,
    pub stiffness: JointArray<f32>,
    pub position: JointArray<f32>,
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

// pub struct DeltaFeet {
//     pub llift: f32,
//     pub rlift: f32,
//     pub loffsets: FootOffsets,
//     pub roffsets: FootOffsets,
// }

const TORSO_SHIFT_OFFSET: f32 = 0.013;
const WALK_HIP_HEIGHT: f32 = 0.185;

enum WalkingFlag {
    L,
    R,
}

struct WalkingParams {
    llift: f32,
    rlift: f32,
    loffsets: FootOffsets,
    roffsets: FootOffsets,
}

impl WalkingParams {
    fn set_vals(&mut self, flag: &mut WalkingFlag) -> () {
        match flag {
            WalkingFlag::L => {
                self.llift = 0.05;
                self.rlift = 0.0;
                self.loffsets = FootOffsets::zero();
                self.roffsets = FootOffsets::zero();
            }
            WalkingFlag::R => {
                self.llift = 0.0;
                self.rlift = 0.05;
                self.loffsets = FootOffsets::zero();
                self.roffsets = FootOffsets::zero();
            },
        }
    }
}

#[system(NaoState)]
pub async fn run_ik_new(
    position: &mut JointArray<f32>,
    ) {
    let mut flag: WalkingFlag = WalkingFlag::R;
    let mut walking_params = WalkingParams {
        llift : 0.0,
        rlift : 0.0,
        loffsets : FootOffsets::zero(),
        roffsets : FootOffsets::zero(),
    };

    for i in 0..1_000 {
        if !(i % 200 == 0) {
            continue;
        }
        // flag = !flag; //toggle side or smthn
        flag = match flag {
            WalkingFlag::L => WalkingFlag::R,
            WalkingFlag::R => WalkingFlag::L,
        };

        walking_params.set_vals(&mut flag);

        let left_foot_to_torso = calculate_foot_to_robot(
            Side::Left,
            walking_params.loffsets,
            0f32, //turn left-right?
            walking_params.llift,
            TORSO_SHIFT_OFFSET,
            WALK_HIP_HEIGHT,
        );
        let right_foot_to_torso = calculate_foot_to_robot(
            Side::Right,
            walking_params.roffsets,
            0f32, //turn left-right?
            walking_params.rlift,
            TORSO_SHIFT_OFFSET,
            WALK_HIP_HEIGHT,
        );

        let (possible, left_leg, right_leg) =
            kinematics::inverse::leg_angles(left_foot_to_torso, right_foot_to_torso);


        if !possible {
            warn!("Impossible move!");
        } else {
            // println!("IK results: {possible}, left: {left_leg:?}, right: {right_leg:?}");
            println!("MOVE: {i}")
        }

        *position = JointArray::builder()
            .left_leg_joints(left_leg)
            .right_leg_joints(right_leg)
            .build();
    }
}

#[system(NaoState)]
pub async fn run_ik_old(position: &mut JointArray<f32>) {
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

    // println!("IK results: {possible}, left: {left_leg:?}, right: {right_leg:?}");

    if !possible {
        warn!("Impossible move!");
    }

    *position = JointArray::builder()
        .left_leg_joints(left_leg)
        .right_leg_joints(right_leg)
        .build();
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
        Side::Left => Isometry3::from(kinematics::bones::ROBOT_TO_LEFT_PELVIS),
        Side::Right => Isometry3::from(kinematics::bones::ROBOT_TO_RIGHT_PELVIS),
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
