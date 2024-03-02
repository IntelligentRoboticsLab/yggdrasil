use std::f32::consts::FRAC_PI_4;

use nalgebra::{Isometry3, Translation, Vector3};
use nidhogg::types::{HeadJoints, JointArray, LeftArmJoints, LeftLegJoints};

use super::robot_dimensions;

pub struct RobotKinematics {
    neck_to_robot: Isometry3<f32>,
    head_to_robot: Isometry3<f32>,
    pub left_pelvis_to_robot: Isometry3<f32>,
    pub left_hip_to_robot: Isometry3<f32>,
    pub left_thigh_to_robot: Isometry3<f32>,
    pub left_tibia_to_robot: Isometry3<f32>,
    pub left_ankle_to_robot: Isometry3<f32>,
    pub left_foot_to_robot: Isometry3<f32>,
    pub left_sole_to_robot: Isometry3<f32>,
}

impl From<JointArray<f32>> for RobotKinematics {
    fn from(joints: JointArray<f32>) -> Self {
        let head_joints: HeadJoints<f32> = joints.head_joints().clone();
        let neck_to_robot = neck_to_robot(head_joints);
        let head_to_robot = neck_to_robot * head_to_neck(joints.head);
    }
}

pub fn neck_to_robot(joints: &HeadJoints<&f32>) -> Isometry3<f32> {
    Translation::from(robot_dimensions::ROBOT_TO_NECK)
        * Isometry3::rotation(Vector3::z() * *joints.yaw)
}

pub fn head_to_neck(joints: HeadJoints<f32>) -> Isometry3<f32> {
    Isometry3::rotation(Vector3::y() * joints.pitch)
}

pub fn left_shoulder_to_robot(joints: LeftArmJoints<f32>) -> Isometry3<f32> {
    Translation::from(robot_dimensions::ROBOT_TO_LEFT_SHOULDER)
        * Isometry3::rotation(Vector3::y() * joints.shoulder_pitch)
}

pub fn left_upper_arm_to_shoulder(joints: LeftArmJoints<f32>) -> Isometry3<f32> {
    Isometry3::rotation(Vector3::z() * joints.shoulder_roll)
}

pub fn left_elbow_to_left_upper_arm(joints: LeftArmJoints<f32>) -> Isometry3<f32> {
    Translation::from(robot_dimensions::LEFT_SHOULDER_TO_LEFT_ELBOW)
        * Isometry3::rotation(Vector3::x() * joints.elbow_yaw)
}

pub fn left_under_arm_to_elbow(joints: LeftArmJoints<f32>) -> Isometry3<f32> {
    Isometry3::rotation(Vector3::z() * joints.elbow_roll)
}

pub fn left_wrist_to_under_arm(joints: LeftArmJoints<f32>) -> Isometry3<f32> {
    Translation::from(robot_dimensions::ELBOW_TO_WRIST)
        * Isometry3::rotation(Vector3::x() * joints.wrist_yaw)
}

pub fn left_pelvis_to_robot(joints: LeftLegJoints<f32>) -> Isometry3<f32> {
    // The pelvis joint controls both the yaw and pitch of the pelvis, so we correct for this
    // by applying a 45 degree roll to the pelvis, then applying the yaw and pitch rotations.
    // And then we go back to the original orientation by applying a -45 degree roll.
    Translation::from(robot_dimensions::ROBOT_TO_LEFT_PELVIS)
        * Isometry3::rotation(Vector3::x() * FRAC_PI_4)
        * Isometry3::rotation(Vector3::z() * -joints.hip_yaw_pitch) // Then apply the hip yaw pitch rotation
        * Isometry3::rotation(Vector3::x() * -FRAC_PI_4)
}

pub fn left_hip_to_left_pelvis(joints: LeftLegJoints<f32>) -> Isometry3<f32> {
    Isometry3::rotation(Vector3::x() * joints.hip_roll)
}

pub fn left_upper_leg_to_left_hip(joints: LeftLegJoints<f32>) -> Isometry3<f32> {
    Isometry3::rotation(Vector3::y() * joints.hip_pitch)
}

pub fn left_knee_to_left_upper_leg(joints: LeftLegJoints<f32>) -> Isometry3<f32> {
    Translation::from(robot_dimensions::HIP_TO_KNEE)
        * Isometry3::rotation(Vector3::y() * joints.knee_pitch)
}

pub fn left_ankle_to_left_knee(joints: LeftLegJoints<f32>) -> Isometry3<f32> {
    Translation::from(robot_dimensions::KNEE_TO_ANKLE)
        * Isometry3::rotation(Vector3::y() * joints.ankle_pitch)
}

pub fn left_foot_to_left_ankle(joints: LeftLegJoints<f32>) -> Isometry3<f32> {
    Isometry3::rotation(Vector3::x() * joints.ankle_roll)
}
