use crate::{
    behavior::engine::in_behavior,
    sensor::falling::{FallDirection, FallState},
};
use bevy::prelude::*;
use nidhogg::{
    types::{
        ArmJoints, FillExt, HeadJoints, LeftArmJoints, LeftLegJoints, LegJoints, RightArmJoints,
        RightLegJoints,
    },
    NaoState,
};

use crate::motion::keyframe::{lerp_arms, lerp_legs};
use crate::{
    behavior::engine::{Behavior, BehaviorState},
    nao::{NaoManager, Priority},
};

/// Behavior used for preventing damage when the robot is in a falling state.
/// This behavior can only be exited from once the robot is lying down.
///
/// # Notes
/// - Currently, the damage prevention is still quite rudimentary, only
///   unstiffing the joints of the robot and making the head stiff.
///   In future this will be accompanied by an appropriate safe falling
///   position.
/// - If the robot incorrectly assumes it is in a falling state yet
///   will not be lying down, the robot will kinda get "soft-locked".
///   Only by unstiffing the robot will it return to normal.
///   This should not be the case often however, once the falling filter
///   is more advanced.
#[derive(Resource, Copy, Clone, Debug, PartialEq)]
pub struct CatchFall;

impl Behavior for CatchFall {
    const STATE: BehaviorState = BehaviorState::CatchFall;
}

pub struct CatchFallBehaviorPlugin;
impl Plugin for CatchFallBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, catch_fall.run_if(in_behavior::<CatchFall>));
    }
}

const LEG_JOINTS_FORWARD_FALL: LegJoints<f32> = LegJoints {
    left_leg: LeftLegJoints {
        hip_yaw_pitch: 0.029187918,
        hip_roll: -0.323632,
        hip_pitch: -0.742414,
        knee_pitch: 2.1521602,
        ankle_pitch: -1.2333779,
        ankle_roll: 0.16264606,
    },
    right_leg: RightLegJoints {
        hip_roll: 0.07060599,
        hip_pitch: -0.28536606,
        knee_pitch: 1.4373999,
        ankle_pitch: -1.142788,
        ankle_roll: -0.024502039,
    },
};

const ARM_JOINTS_FORWARD_FALL: ArmJoints<f32> = ArmJoints {
    right_arm: RightArmJoints {
        shoulder_pitch: 1.2441158,
        shoulder_roll: -0.110489845,
        elbow_yaw: 0.050580025,
        elbow_roll: 0.81306195,
        wrist_yaw: 1.5354921,
        hand: 0.32959998,
    },
    left_arm: LeftArmJoints {
        shoulder_pitch: 1.035408,
        shoulder_roll: -0.116626024,
        elbow_yaw: 0.22238803,
        elbow_roll: -0.67184997,
        wrist_yaw: -0.834538,
        hand: 0.4076,
    },
};

const LEG_JOINTS_SIDE_FALL: LegJoints<f32> = LegJoints {
    left_leg: LeftLegJoints {
        hip_yaw_pitch: 0.0,
        hip_roll: 0.003109932,
        hip_pitch: -0.9310961,
        knee_pitch: 2.12,
        ankle_pitch: -1.18,
        ankle_roll: 0.0015759468,
    },
    right_leg: RightLegJoints {
        hip_roll: 0.0,
        hip_pitch: -0.9403839,
        knee_pitch: 2.12,
        ankle_pitch: -1.18,
        ankle_roll: 0.0015759468,
    },
};

const ARM_JOINTS_SIDE_FALL: ArmJoints<f32> = ArmJoints {
    left_arm: LeftArmJoints {
        shoulder_pitch: 1.5585021,
        shoulder_roll: 0.6304321,
        elbow_yaw: -0.012313843,
        elbow_roll: -0.92496014,
        wrist_yaw: -1.6429558,
        hand: 0.37,
    },
    right_arm: LeftArmJoints {
        shoulder_pitch: 1.5585021,
        shoulder_roll: -0.6304321,
        elbow_yaw: 0.012313843,
        elbow_roll: 0.92496014,
        wrist_yaw: 1.6429558,
        hand: 0.37,
    },
};

const LEG_JOINTS_BACKWARD_FALL: LegJoints<f32> = LegJoints {
    left_leg: LeftLegJoints {
        hip_yaw_pitch: -0.07665801,
        hip_roll: -0.19324207,
        hip_pitch: -1.3836261,
        knee_pitch: 1.57691,
        ankle_pitch: -0.70568204,
        ankle_roll: -0.024502039,
    },
    right_leg: RightLegJoints {
        hip_roll: 0.026119947,
        hip_pitch: -1.5463142,
        knee_pitch: 1.5647221,
        ankle_pitch: -0.645772,
        ankle_roll: -0.026036024,
    },
};

const ARM_JOINTS_BACKWARD_FALL: ArmJoints<f32> = ArmJoints {
    left_arm: LeftArmJoints {
        shoulder_pitch: 2.0539842,
        shoulder_roll: 0.19937801,
        elbow_yaw: -4.196167e-5,
        elbow_roll: -1.194944,
        wrist_yaw: -1.083046,
        hand: 0.4076,
    },
    right_arm: RightArmJoints {
        shoulder_pitch: 2.1261659,
        shoulder_roll: -0.24548197,
        elbow_yaw: -0.046061993,
        elbow_roll: 1.2625241,
        wrist_yaw: 1.550832,
        hand: 0.32919997,
    },
};

pub fn catch_fall(
    mut nao_manager: ResMut<NaoManager>,
    nao_state: ResMut<NaoState>,

    fall_state: Res<FallState>,
) {
    //eprintln!("{:?}", nao_state.position.left_leg_joints());
    //eprintln!("{:?}", nao_state.position.right_leg_joints());
    //eprintln!("right_arm: {:?}", nao_state.position.right_arm_joints());
    //eprintln!("left_arm: {:?}", nao_state.position.left_arm_joints());

    if let FallState::Falling(fall_direction) = fall_state.as_ref() {
        match fall_direction {
            FallDirection::Forwards => {
                let target_leg_joints = lerp_legs(
                    &nao_state.position.leg_joints(),
                    &LEG_JOINTS_FORWARD_FALL,
                    0.5,
                );
                let target_arm_joints = lerp_arms(
                    &nao_state.position.arm_joints(),
                    &ARM_JOINTS_FORWARD_FALL,
                    0.5,
                );

                nao_manager.set_legs(target_leg_joints, LegJoints::fill(0.1), Priority::Critical);
                nao_manager.set_head(
                    HeadJoints {
                        yaw: 0.0,
                        pitch: -0.6,
                    },
                    HeadJoints::fill(0.3),
                    Priority::Critical,
                );
                nao_manager.set_arms(target_arm_joints, ArmJoints::fill(0.1), Priority::Critical);
            }
            FallDirection::Left | FallDirection::Right => {
                let target_leg_joints =
                    lerp_legs(&nao_state.position.leg_joints(), &LEG_JOINTS_SIDE_FALL, 0.5);
                let target_arm_joints =
                    lerp_arms(&nao_state.position.arm_joints(), &ARM_JOINTS_SIDE_FALL, 0.5);

                nao_manager.set_legs(target_leg_joints, LegJoints::fill(0.1), Priority::Critical);
                nao_manager.set_arms(target_arm_joints, ArmJoints::fill(0.1), Priority::Critical);
                nao_manager.set_head(
                    HeadJoints::default(),
                    HeadJoints::fill(0.3),
                    Priority::Critical,
                );
            }
            FallDirection::Backwards => {
                let target_leg_joints = lerp_legs(
                    &nao_state.position.leg_joints(),
                    &LEG_JOINTS_BACKWARD_FALL,
                    0.6,
                );
                let target_arm_joints = lerp_arms(
                    &nao_state.position.arm_joints(),
                    &ARM_JOINTS_BACKWARD_FALL,
                    0.5,
                );

                nao_manager.set_head(
                    HeadJoints {
                        yaw: 0.0,
                        pitch: 0.6,
                    },
                    HeadJoints::fill(0.3),
                    Priority::Critical,
                );
                nao_manager.set_legs(target_leg_joints, LegJoints::fill(0.1), Priority::Critical);
                nao_manager.set_arms(target_arm_joints, ArmJoints::fill(0.1), Priority::Critical);
            }
        }
    };
}
