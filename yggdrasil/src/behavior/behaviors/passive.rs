use crate::{
    behavior::engine::{Behavior, Context},
    nao::manager::{NaoManager, Priority},
};
use nidhogg::types::{
    color, ArmJoints, FillExt, HeadJoints, JointArray, LeftArmJoints, LeftLegJoints, LegJoints,
    RightArmJoints, RightEye, RightLegJoints,
};

const DEFAULT_PASSIVE_STIFFNESS: f32 = 0.8;
const DEFAULT_PASSIVE_PRIORITY: u32 = 100;

/// This is the default behavior of the robot.
/// In this state the robot does nothing and all motors are turned off.
/// In this state the robot has a blue right eye.
#[derive(Copy, Clone, Debug, Default)]
pub struct Passive;

impl Behavior for Passive {
    fn execute(&mut self, context: Context, nao_manager: &mut NaoManager) {
        // Turns off motors
        let JointArray {
            head_yaw,
            head_pitch,
            left_shoulder_pitch,
            left_shoulder_roll,
            left_elbow_yaw,
            left_elbow_roll,
            left_wrist_yaw,
            left_hip_yaw_pitch,
            left_hip_roll,
            left_hip_pitch,
            left_knee_pitch,
            left_ankle_pitch,
            left_ankle_roll,
            right_shoulder_pitch,
            right_shoulder_roll,
            right_elbow_yaw,
            right_elbow_roll,
            right_wrist_yaw,
            right_hip_roll,
            right_hip_pitch,
            right_knee_pitch,
            right_ankle_pitch,
            right_ankle_roll,
            left_hand,
            right_hand,
        } = context.robot_info.initial_joint_position;
        nao_manager.set_head(
            HeadJoints {
                yaw: head_yaw,
                pitch: head_pitch,
            },
            HeadJoints::fill(DEFAULT_PASSIVE_STIFFNESS),
            Priority::Custom(DEFAULT_PASSIVE_PRIORITY),
        );

        nao_manager.set_arms(
            ArmJoints {
                left_arm: LeftArmJoints {
                    shoulder_pitch: left_shoulder_pitch,
                    shoulder_roll: left_shoulder_roll,
                    elbow_yaw: left_elbow_yaw,
                    elbow_roll: left_elbow_roll,
                    wrist_yaw: left_wrist_yaw,
                    hand: left_hand,
                },
                right_arm: RightArmJoints {
                    shoulder_pitch: right_shoulder_pitch,
                    shoulder_roll: right_shoulder_roll,
                    elbow_yaw: right_elbow_yaw,
                    elbow_roll: right_elbow_roll,
                    wrist_yaw: right_wrist_yaw,
                    hand: right_hand,
                },
            },
            ArmJoints::fill(DEFAULT_PASSIVE_STIFFNESS),
            Priority::Custom(DEFAULT_PASSIVE_PRIORITY),
        );

        nao_manager.set_legs(
            LegJoints {
                left_leg: LeftLegJoints {
                    hip_yaw_pitch: left_hip_yaw_pitch,
                    hip_roll: left_hip_roll,
                    hip_pitch: left_hip_pitch,
                    knee_pitch: left_knee_pitch,
                    ankle_pitch: left_ankle_pitch,
                    ankle_roll: left_ankle_roll,
                },
                right_leg: RightLegJoints {
                    hip_roll: right_hip_roll,
                    hip_pitch: right_hip_pitch,
                    knee_pitch: right_knee_pitch,
                    ankle_pitch: right_ankle_pitch,
                    ankle_roll: right_ankle_roll,
                },
            },
            LegJoints::fill(DEFAULT_PASSIVE_STIFFNESS),
            Priority::Custom(DEFAULT_PASSIVE_PRIORITY),
        );

        // Makes right eye blue.
        nao_manager.set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::default());
    }
}
