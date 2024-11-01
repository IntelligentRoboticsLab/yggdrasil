//! The forward kinematics for the robot.
//!
//! This module is based on the implementation in the HULKs 2023 code release.

use bevy::prelude::*;
use nalgebra as na;

use std::f32::consts::FRAC_1_SQRT_2;

use nidhogg::types::JointArray;
use spatial::types::Isometry3;
use super::{robot_dimensions::*, spaces::*};

#[derive(spatial::Transform, Resource, Debug)]
pub struct Kinematics {
    pub head_to_neck: Isometry3<Head, Neck>,
    pub neck_to_robot: Isometry3<Neck, Robot>,
    pub torso_to_robot: Isometry3<Torso, Robot>,
    pub left_shoulder_to_robot: Isometry3<Shoulder<Left>, Robot>,
    pub left_upper_arm_to_left_shoulder: Isometry3<UpperArm<Left>, Shoulder<Left>>,
    pub left_elbow_to_left_upper_arm: Isometry3<Elbow<Left>, UpperArm<Left>>,
    pub left_forearm_to_left_elbow: Isometry3<Forearm<Left>, Elbow<Left>>,
    pub left_wrist_to_left_forearm: Isometry3<Wrist<Left>, Forearm<Left>>,
    pub right_shoulder_to_robot: Isometry3<Shoulder<Right>, Robot>,
    pub right_upper_arm_to_right_shoulder: Isometry3<UpperArm<Right>, Shoulder<Right>>,
    pub right_elbow_to_right_upper_arm: Isometry3<Elbow<Right>, UpperArm<Right>>,
    pub right_forearm_to_right_elbow: Isometry3<Forearm<Right>, Elbow<Right>>,
    pub right_wrist_to_right_forearm: Isometry3<Wrist<Right>, Forearm<Right>>,
    pub left_pelvis_to_robot: Isometry3<Pelvis<Left>, Robot>,
    pub left_hip_to_left_pelvis: Isometry3<Hip<Left>, Pelvis<Left>>,
    pub left_thigh_to_left_hip: Isometry3<Thigh<Left>, Hip<Left>>,
    pub left_tibia_to_left_thigh: Isometry3<Tibia<Left>, Thigh<Left>>,
    pub left_ankle_to_left_tibia: Isometry3<Ankle<Left>, Tibia<Left>>,
    pub left_foot_to_left_ankle: Isometry3<Foot<Left>, Ankle<Left>>,
    pub left_sole_to_left_foot: Isometry3<Sole<Left>, Foot<Left>>,
    pub right_pelvis_to_robot: Isometry3<Pelvis<Right>, Robot>,
    pub right_hip_to_right_pelvis: Isometry3<Hip<Right>, Pelvis<Right>>,
    pub right_thigh_to_right_hip: Isometry3<Thigh<Right>, Hip<Right>>,
    pub right_tibia_to_right_thigh: Isometry3<Tibia<Right>, Thigh<Right>>,
    pub right_ankle_to_right_tibia: Isometry3<Ankle<Right>, Tibia<Right>>,
    pub right_foot_to_right_ankle: Isometry3<Foot<Right>, Ankle<Right>>,
    pub right_sole_to_right_foot: Isometry3<Sole<Right>, Foot<Right>>,
}

impl From<&JointArray<f32>> for Kinematics {
    fn from(joints: &JointArray<f32>) -> Self {
        Self {
            // Upper body
            head_to_neck: na::Isometry3::rotation(
                na::Vector3::y() * joints.head_pitch
            ).into(),
            neck_to_robot: na::Isometry3::new(
                ROBOT_TO_NECK,
                na::Vector3::z() * joints.head_yaw
            ).into(),
            torso_to_robot: na::Isometry3::from(
                ROBOT_TO_TORSO,
            ).into(),
            // Left upper body
            left_shoulder_to_robot: na::Isometry3::new(
                ROBOT_TO_LEFT_SHOULDER,
                na::Vector3::y() * joints.left_shoulder_pitch
            ).into(),
            left_upper_arm_to_left_shoulder: na::Isometry3::rotation(
                na::Vector3::z() * joints.left_shoulder_roll
            ).into(),
            left_elbow_to_left_upper_arm: na::Isometry3::new(
                LEFT_SHOULDER_TO_LEFT_ELBOW,
                na::Vector3::x() * joints.left_elbow_yaw
            ).into(),
            left_forearm_to_left_elbow: na::Isometry3::rotation(
                na::Vector3::z() * joints.left_elbow_roll
            ).into(),
            left_wrist_to_left_forearm: na::Isometry3::new(
                ELBOW_TO_WRIST,
                na::Vector3::x() * joints.left_wrist_yaw
            ).into(),
            // Right upper body
            right_shoulder_to_robot: na::Isometry3::new(
                ROBOT_TO_RIGHT_SHOULDER,
                na::Vector3::y() * joints.right_shoulder_pitch
            ).into(),
            right_upper_arm_to_right_shoulder: na::Isometry3::rotation(
                na::Vector3::z() * joints.right_shoulder_roll
            ).into(),
            right_elbow_to_right_upper_arm: na::Isometry3::new(
                RIGHT_SHOULDER_TO_RIGHT_ELBOW,
                na::Vector3::x() * joints.right_elbow_yaw
            ).into(),
            right_forearm_to_right_elbow: na::Isometry3::rotation(
                na::Vector3::z() * joints.right_elbow_roll
            ).into(),
            right_wrist_to_right_forearm: na::Isometry3::new(
                ELBOW_TO_WRIST,
                na::Vector3::x() * joints.right_wrist_yaw
            ).into(),
            // Left lower body
            left_pelvis_to_robot: na::Isometry3::new(
                ROBOT_TO_LEFT_PELVIS,
                na::Vector3::new(0., FRAC_1_SQRT_2, -FRAC_1_SQRT_2) * joints.left_hip_yaw_pitch
            ).into(),
            left_hip_to_left_pelvis: na::Isometry3::rotation(
                na::Vector3::x() * joints.left_hip_roll
            ).into(),
            left_thigh_to_left_hip: na::Isometry3::rotation(
                na::Vector3::y() * joints.left_hip_pitch
            ).into(),
            left_tibia_to_left_thigh: na::Isometry3::new(
                HIP_TO_KNEE,
                na::Vector3::y() * joints.left_knee_pitch
            ).into(),
            left_ankle_to_left_tibia: na::Isometry3::new(
                KNEE_TO_ANKLE,
                na::Vector3::y() * joints.left_ankle_pitch
            ).into(),
            left_foot_to_left_ankle: na::Isometry3::rotation(
                na::Vector3::x() * joints.left_ankle_roll
            ).into(),
            left_sole_to_left_foot: na::Isometry3::from(
                ANKLE_TO_SOLE
            ).into(),
            // Right lower body
            right_pelvis_to_robot: na::Isometry3::new(
                ROBOT_TO_RIGHT_PELVIS,
                na::Vector3::new(0., FRAC_1_SQRT_2, FRAC_1_SQRT_2) * joints.left_hip_yaw_pitch
            ).into(),
            right_hip_to_right_pelvis: na::Isometry3::rotation(
                na::Vector3::x() * joints.right_hip_roll
            ).into(),
            right_thigh_to_right_hip: na::Isometry3::rotation(
                na::Vector3::y() * joints.right_hip_pitch
            ).into(),
            right_tibia_to_right_thigh: na::Isometry3::new(
                HIP_TO_KNEE,
                na::Vector3::y() * joints.right_knee_pitch
            ).into(),
            right_ankle_to_right_tibia: na::Isometry3::new(
                KNEE_TO_ANKLE,
                na::Vector3::y() * joints.right_ankle_pitch
            ).into(),
            right_foot_to_right_ankle: na::Isometry3::rotation(
                na::Vector3::x() * joints.right_ankle_roll
            ).into(),
            right_sole_to_right_foot: na::Isometry3::from(
                ANKLE_TO_SOLE
            ).into(),
        }
    }
}
