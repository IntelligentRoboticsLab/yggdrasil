use crate::{
    behavior::engine::in_behavior,
    sensor::falling::{FallDirection, FallState},
};
use bevy::prelude::*;
use nidhogg::{
    NaoState,
    types::{
        ArmJoints, FillExt, HeadJoints, LeftArmJoints, LeftLegJoints, LegJoints, RightArmJoints,
        RightLegJoints,
    },
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
        hip_yaw_pitch: -0.001_492_023_5,
        hip_roll: -0.058_249_95,
        hip_pitch: -0.952_572_1,
        knee_pitch: 1.799_340_1,
        ankle_pitch: -1.175_086,
        ankle_roll: -0.013_764_143,
    },
    right_leg: RightLegJoints {
        hip_roll: 0.004_643_917,
        hip_pitch: -1.603_072_2,
        knee_pitch: 2.155_312,
        ankle_pitch: -1.222_556_1,
        ankle_roll: 0.006_177_902,
    },
};

const ARM_JOINTS_FORWARD_FALL: ArmJoints<f32> = ArmJoints {
    right_arm: RightArmJoints {
        shoulder_pitch: 1.244_115_8,
        shoulder_roll: -0.110_489_845,
        elbow_yaw: 0.050_580_025,
        elbow_roll: 0.813_061_95,
        wrist_yaw: 1.535_492_1,
        hand: 0.329_599_98,
    },
    left_arm: LeftArmJoints {
        shoulder_pitch: 1.035_408,
        shoulder_roll: -0.116_626_024,
        elbow_yaw: 0.222_388_03,
        elbow_roll: -0.671_849_97,
        wrist_yaw: -0.834_538,
        hand: 0.407_6,
    },
};

const LEG_JOINTS_SIDE_FALL: LegJoints<f32> = LegJoints {
    left_leg: LeftLegJoints {
        hip_yaw_pitch: 0.0,
        hip_roll: 0.003_109_932,
        hip_pitch: -0.931_096_1,
        knee_pitch: 2.12,
        ankle_pitch: -1.18,
        ankle_roll: 0.001_575_946_8,
    },
    right_leg: RightLegJoints {
        hip_roll: 0.0,
        hip_pitch: -0.940_383_9,
        knee_pitch: 2.12,
        ankle_pitch: -1.18,
        ankle_roll: 0.001_575_946_8,
    },
};

const ARM_JOINTS_SIDE_FALL: ArmJoints<f32> = ArmJoints {
    left_arm: LeftArmJoints {
        shoulder_pitch: 1.558_502_1,
        shoulder_roll: 0.630_432_1,
        elbow_yaw: -0.012_313_843,
        elbow_roll: -0.924_960_14,
        wrist_yaw: -1.642_955_8,
        hand: 0.37,
    },
    right_arm: LeftArmJoints {
        shoulder_pitch: 1.558_502_1,
        shoulder_roll: -0.630_432_1,
        elbow_yaw: 0.012_313_843,
        elbow_roll: 0.924_960_14,
        wrist_yaw: 1.642_955_8,
        hand: 0.37,
    },
};

const LEG_JOINTS_BACKWARD_FALL: LegJoints<f32> = LegJoints {
    left_leg: LeftLegJoints {
        hip_yaw_pitch: -0.076_658_01,
        hip_roll: -0.193_242_07,
        hip_pitch: -1.383_626_1,
        knee_pitch: 1.576_91,
        ankle_pitch: -0.705_682_04,
        ankle_roll: -0.024_502_039,
    },
    right_leg: RightLegJoints {
        hip_roll: 0.026_119_947,
        hip_pitch: -1.546_314_2,
        knee_pitch: 1.564_722_1,
        ankle_pitch: -0.645_772,
        ankle_roll: -0.026_036_024,
    },
};

const ARM_JOINTS_BACKWARD_FALL: ArmJoints<f32> = ArmJoints {
    left_arm: LeftArmJoints {
        shoulder_pitch: 2.053_984_2,
        shoulder_roll: 0.199_378_01,
        elbow_yaw: -0.04,
        elbow_roll: -1.194_944,
        wrist_yaw: -1.083_046,
        hand: 0.4076,
    },
    right_arm: RightArmJoints {
        shoulder_pitch: 2.126_165_9,
        shoulder_roll: -0.245_481_97,
        elbow_yaw: -0.046_061_993,
        elbow_roll: 1.262_524_1,
        wrist_yaw: 1.550_832,
        hand: 0.329_199_97,
    },
};

fn catch_fall(
    mut nao_manager: ResMut<NaoManager>,
    nao_state: ResMut<NaoState>,

    fall_state: Res<FallState>,
) {
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
                    0.6,
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
    }
}
