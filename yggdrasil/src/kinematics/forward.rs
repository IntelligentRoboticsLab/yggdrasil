//! The forward kinematics for the robot.
//!
//! This module is based on the implementation in the HULKs 2023 code release.

use bevy::prelude::*;
use nalgebra as na;

use std::f32::consts::FRAC_1_SQRT_2;

use super::{dimensions::*, spaces::*};
use nidhogg::types::JointArray;
use spatial::{types::Isometry3, InSpace, Space, SpaceOver, Transform};

#[derive(Debug, Resource, Transform)]
pub struct Kinematics {
    pub head_to_neck: Isometry3<Head, Neck>,
    pub neck_to_robot: Isometry3<Neck, Robot>,
    pub torso_to_robot: Isometry3<Torso, Robot>,
    pub left_shoulder_to_robot: Isometry3<Shoulder<Left>, Robot>,
    pub left_upper_arm_to_shoulder: Isometry3<UpperArm<Left>, Shoulder<Left>>,
    pub left_elbow_to_upper_arm: Isometry3<Elbow<Left>, UpperArm<Left>>,
    pub left_forearm_to_elbow: Isometry3<Forearm<Left>, Elbow<Left>>,
    pub left_wrist_to_forearm: Isometry3<Wrist<Left>, Forearm<Left>>,
    pub right_shoulder_to_robot: Isometry3<Shoulder<Right>, Robot>,
    pub right_upper_arm_to_shoulder: Isometry3<UpperArm<Right>, Shoulder<Right>>,
    pub right_elbow_to_upper_arm: Isometry3<Elbow<Right>, UpperArm<Right>>,
    pub right_forearm_to_elbow: Isometry3<Forearm<Right>, Elbow<Right>>,
    pub right_wrist_to_forearm: Isometry3<Wrist<Right>, Forearm<Right>>,
    pub left_pelvis_to_robot: Isometry3<Pelvis<Left>, Robot>,
    pub left_hip_to_pelvis: Isometry3<Hip<Left>, Pelvis<Left>>,
    pub left_thigh_to_hip: Isometry3<Thigh<Left>, Hip<Left>>,
    pub left_tibia_to_thigh: Isometry3<Tibia<Left>, Thigh<Left>>,
    pub left_ankle_to_tibia: Isometry3<Ankle<Left>, Tibia<Left>>,
    pub left_foot_to_ankle: Isometry3<Foot<Left>, Ankle<Left>>,
    pub left_sole_to_foot: Isometry3<Sole<Left>, Foot<Left>>,
    pub right_pelvis_to_robot: Isometry3<Pelvis<Right>, Robot>,
    pub right_hip_to_pelvis: Isometry3<Hip<Right>, Pelvis<Right>>,
    pub right_thigh_to_hip: Isometry3<Thigh<Right>, Hip<Right>>,
    pub right_tibia_to_thigh: Isometry3<Tibia<Right>, Thigh<Right>>,
    pub right_ankle_to_tibia: Isometry3<Ankle<Right>, Tibia<Right>>,
    pub right_foot_to_ankle: Isometry3<Foot<Right>, Ankle<Right>>,
    pub right_sole_to_foot: Isometry3<Sole<Right>, Foot<Right>>,
}

impl Kinematics {
    #[must_use]
    pub fn transform<T, S1, S2>(&self, x: &InSpace<T, S1>) -> InSpace<T, S2>
    where
        S1: Space + SpaceOver<T>,
        S2: Space + SpaceOver<T>,
        Self: Transform<T, T, S1, S2>,
    {
        Transform::transform(self, x)
    }

    #[must_use]
    pub fn to_robot<T, S>(&self, x: &InSpace<T, S>) -> InSpace<T, Robot>
    where
        S: Space + SpaceOver<T>,
        Robot: SpaceOver<T>,
        Self: Transform<T, T, S, Robot>,
    {
        self.transform(x)
    }

    #[must_use]
    pub fn isometry<S1, S2>(&self) -> Isometry3<S1, S2>
    where
        S1: Space + SpaceOver<na::Isometry3<f32>>,
        S2: Space + SpaceOver<na::Isometry3<f32>>,
        Self: Transform<na::Isometry3<f32>, na::Isometry3<f32>, S1, S2>,
    {
        self.transform(&na::Isometry3::identity().into())
            .inner
            .into()
    }

    #[must_use]
    pub fn head_to_neck(head_pitch: f32) -> Isometry3<Head, Neck> {
        na::Isometry3::rotation(na::Vector3::y() * head_pitch).into()
    }

    #[must_use]
    pub fn neck_to_robot(head_yaw: f32) -> Isometry3<Neck, Robot> {
        na::Isometry3::new(ROBOT_TO_NECK, na::Vector3::z() * head_yaw).into()
    }

    #[must_use]
    pub fn torso_to_robot() -> Isometry3<Torso, Robot> {
        na::Isometry3::from(ROBOT_TO_TORSO).into()
    }

    #[must_use]
    pub fn left_shoulder_to_robot(left_shoulder_pitch: f32) -> Isometry3<Shoulder<Left>, Robot> {
        na::Isometry3::new(
            ROBOT_TO_LEFT_SHOULDER,
            na::Vector3::y() * left_shoulder_pitch,
        )
        .into()
    }

    #[must_use]
    pub fn left_upper_arm_to_shoulder(
        left_shoulder_roll: f32,
    ) -> Isometry3<UpperArm<Left>, Shoulder<Left>> {
        na::Isometry3::rotation(na::Vector3::z() * left_shoulder_roll).into()
    }

    #[must_use]
    pub fn left_elbow_to_upper_arm(left_elbow_yaw: f32) -> Isometry3<Elbow<Left>, UpperArm<Left>> {
        na::Isometry3::new(
            LEFT_SHOULDER_TO_LEFT_ELBOW,
            na::Vector3::x() * left_elbow_yaw,
        )
        .into()
    }

    #[must_use]
    pub fn left_forearm_to_elbow(left_elbow_roll: f32) -> Isometry3<Forearm<Left>, Elbow<Left>> {
        na::Isometry3::rotation(na::Vector3::z() * left_elbow_roll).into()
    }

    #[must_use]
    pub fn left_wrist_to_forearm(left_wrist_yaw: f32) -> Isometry3<Wrist<Left>, Forearm<Left>> {
        na::Isometry3::new(ELBOW_TO_WRIST, na::Vector3::x() * left_wrist_yaw).into()
    }

    #[must_use]
    pub fn right_shoulder_to_robot(right_shoulder_pitch: f32) -> Isometry3<Shoulder<Right>, Robot> {
        na::Isometry3::new(
            ROBOT_TO_RIGHT_SHOULDER,
            na::Vector3::y() * right_shoulder_pitch,
        )
        .into()
    }

    #[must_use]
    pub fn right_upper_arm_to_shoulder(
        right_shoulder_roll: f32,
    ) -> Isometry3<UpperArm<Right>, Shoulder<Right>> {
        na::Isometry3::rotation(na::Vector3::z() * right_shoulder_roll).into()
    }

    #[must_use]
    pub fn right_elbow_to_upper_arm(
        right_elbow_yaw: f32,
    ) -> Isometry3<Elbow<Right>, UpperArm<Right>> {
        na::Isometry3::new(
            RIGHT_SHOULDER_TO_RIGHT_ELBOW,
            na::Vector3::x() * right_elbow_yaw,
        )
        .into()
    }

    #[must_use]
    pub fn right_forearm_to_elbow(
        right_elbow_roll: f32,
    ) -> Isometry3<Forearm<Right>, Elbow<Right>> {
        na::Isometry3::rotation(na::Vector3::z() * right_elbow_roll).into()
    }

    #[must_use]
    pub fn right_wrist_to_forearm(right_wrist_yaw: f32) -> Isometry3<Wrist<Right>, Forearm<Right>> {
        na::Isometry3::new(ELBOW_TO_WRIST, na::Vector3::x() * right_wrist_yaw).into()
    }

    #[must_use]
    pub fn left_pelvis_to_robot(left_hip_yaw_pitch: f32) -> Isometry3<Pelvis<Left>, Robot> {
        na::Isometry3::new(
            ROBOT_TO_LEFT_PELVIS,
            na::Vector3::new(0., FRAC_1_SQRT_2, -FRAC_1_SQRT_2) * left_hip_yaw_pitch,
        )
        .into()
    }

    #[must_use]
    pub fn left_hip_to_pelvis(left_hip_roll: f32) -> Isometry3<Hip<Left>, Pelvis<Left>> {
        na::Isometry3::rotation(na::Vector3::x() * left_hip_roll).into()
    }

    #[must_use]
    pub fn left_thigh_to_hip(left_hip_pitch: f32) -> Isometry3<Thigh<Left>, Hip<Left>> {
        na::Isometry3::rotation(na::Vector3::y() * left_hip_pitch).into()
    }

    #[must_use]
    pub fn left_tibia_to_thigh(left_knee_pitch: f32) -> Isometry3<Tibia<Left>, Thigh<Left>> {
        na::Isometry3::new(HIP_TO_KNEE, na::Vector3::y() * left_knee_pitch).into()
    }

    #[must_use]
    pub fn left_ankle_to_tibia(left_ankle_pitch: f32) -> Isometry3<Ankle<Left>, Tibia<Left>> {
        na::Isometry3::new(KNEE_TO_ANKLE, na::Vector3::y() * left_ankle_pitch).into()
    }

    #[must_use]
    pub fn left_foot_to_ankle(left_ankle_roll: f32) -> Isometry3<Foot<Left>, Ankle<Left>> {
        na::Isometry3::rotation(na::Vector3::x() * left_ankle_roll).into()
    }

    #[must_use]
    pub fn left_sole_to_foot() -> Isometry3<Sole<Left>, Foot<Left>> {
        na::Isometry3::from(ANKLE_TO_SOLE).into()
    }

    #[must_use]
    pub fn right_pelvis_to_robot(left_hip_yaw_pitch: f32) -> Isometry3<Pelvis<Right>, Robot> {
        na::Isometry3::new(
            ROBOT_TO_RIGHT_PELVIS,
            na::Vector3::new(0., FRAC_1_SQRT_2, FRAC_1_SQRT_2) * left_hip_yaw_pitch,
        )
        .into()
    }

    #[must_use]
    pub fn right_hip_to_pelvis(right_hip_roll: f32) -> Isometry3<Hip<Right>, Pelvis<Right>> {
        na::Isometry3::rotation(na::Vector3::x() * right_hip_roll).into()
    }

    #[must_use]
    pub fn right_thigh_to_hip(right_hip_pitch: f32) -> Isometry3<Thigh<Right>, Hip<Right>> {
        na::Isometry3::rotation(na::Vector3::y() * right_hip_pitch).into()
    }

    #[must_use]
    pub fn right_tibia_to_thigh(right_knee_pitch: f32) -> Isometry3<Tibia<Right>, Thigh<Right>> {
        na::Isometry3::new(HIP_TO_KNEE, na::Vector3::y() * right_knee_pitch).into()
    }

    #[must_use]
    pub fn right_ankle_to_tibia(right_ankle_pitch: f32) -> Isometry3<Ankle<Right>, Tibia<Right>> {
        na::Isometry3::new(KNEE_TO_ANKLE, na::Vector3::y() * right_ankle_pitch).into()
    }

    #[must_use]
    pub fn right_foot_to_ankle(right_ankle_roll: f32) -> Isometry3<Foot<Right>, Ankle<Right>> {
        na::Isometry3::rotation(na::Vector3::x() * right_ankle_roll).into()
    }

    #[must_use]
    pub fn right_sole_to_foot() -> Isometry3<Sole<Right>, Foot<Right>> {
        na::Isometry3::from(ANKLE_TO_SOLE).into()
    }
}

impl From<&JointArray<f32>> for Kinematics {
    fn from(joints: &JointArray<f32>) -> Self {
        Self {
            head_to_neck: Self::head_to_neck(joints.head_pitch),
            neck_to_robot: Self::neck_to_robot(joints.head_yaw),
            torso_to_robot: Self::torso_to_robot(),
            left_shoulder_to_robot: Self::left_shoulder_to_robot(joints.left_shoulder_pitch),
            left_upper_arm_to_shoulder: Self::left_upper_arm_to_shoulder(joints.left_shoulder_roll),
            left_elbow_to_upper_arm: Self::left_elbow_to_upper_arm(joints.left_elbow_yaw),
            left_forearm_to_elbow: Self::left_forearm_to_elbow(joints.left_elbow_roll),
            left_wrist_to_forearm: Self::left_wrist_to_forearm(joints.left_wrist_yaw),
            right_shoulder_to_robot: Self::right_shoulder_to_robot(joints.right_shoulder_pitch),
            right_upper_arm_to_shoulder: Self::right_upper_arm_to_shoulder(
                joints.right_shoulder_roll,
            ),
            right_elbow_to_upper_arm: Self::right_elbow_to_upper_arm(joints.right_elbow_yaw),
            right_forearm_to_elbow: Self::right_forearm_to_elbow(joints.right_elbow_roll),
            right_wrist_to_forearm: Self::right_wrist_to_forearm(joints.right_wrist_yaw),
            left_pelvis_to_robot: Self::left_pelvis_to_robot(joints.left_hip_yaw_pitch),
            left_hip_to_pelvis: Self::left_hip_to_pelvis(joints.left_hip_roll),
            left_thigh_to_hip: Self::left_thigh_to_hip(joints.left_hip_pitch),
            left_tibia_to_thigh: Self::left_tibia_to_thigh(joints.left_knee_pitch),
            left_ankle_to_tibia: Self::left_ankle_to_tibia(joints.left_ankle_pitch),
            left_foot_to_ankle: Self::left_foot_to_ankle(joints.left_ankle_roll),
            left_sole_to_foot: Self::left_sole_to_foot(),
            right_pelvis_to_robot: Self::right_pelvis_to_robot(joints.left_hip_yaw_pitch),
            right_hip_to_pelvis: Self::right_hip_to_pelvis(joints.right_hip_roll),
            right_thigh_to_hip: Self::right_thigh_to_hip(joints.right_hip_pitch),
            right_tibia_to_thigh: Self::right_tibia_to_thigh(joints.right_knee_pitch),
            right_ankle_to_tibia: Self::right_ankle_to_tibia(joints.right_ankle_pitch),
            right_foot_to_ankle: Self::right_foot_to_ankle(joints.right_ankle_roll),
            right_sole_to_foot: Self::right_sole_to_foot(),
        }
    }
}

impl Default for Kinematics {
    fn default() -> Self {
        Self::from(&JointArray::default())
    }
}
