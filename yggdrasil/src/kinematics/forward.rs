//! The forward kinematics for the robot.
//!
//! This module is based on the implementation in the HULKs 2023 code release.

use bevy::prelude::*;
use nalgebra as na;

use std::f32::consts::FRAC_1_SQRT_2;

use super::prelude::*;
use nidhogg::types::JointArray;
use spatial::{
    types::{Isometry3, Point3, Vector3},
    InSpace, Space, SpaceOver, Transform,
};

#[derive(Debug, Resource, Transform)]
pub struct Kinematics {
    pub head_to_neck: Isometry3<Head, Neck>,
    pub neck_to_robot: Isometry3<Neck, Robot>,
    pub torso_to_robot: Isometry3<Torso, Robot>,
    pub chest_to_torso: Isometry3<Chest, Torso>,
    pub chest_to_chest_left: Isometry3<Chest, ChestLeft>,
    pub chest_to_chest_right: Isometry3<Chest, ChestRight>,
    pub chest_to_chest_center_left: Isometry3<Chest, ChestCenterLeft>,
    pub chest_to_chest_center_right: Isometry3<Chest, ChestCenterRight>,
    pub left_shoulder_to_robot: Isometry3<LeftShoulder, Robot>,
    pub left_shoulder_cap_to_robot: Isometry3<LeftShoulderCap, Robot>,
    pub left_shoulder_cap_front_to_left_shoulder_cap:
        Isometry3<LeftShoulderCapFront, LeftShoulderCap>,
    pub left_shoulder_cap_back_to_left_shoulder_cap:
        Isometry3<LeftShoulderCapBack, LeftShoulderCap>,
    pub left_upper_arm_to_shoulder: Isometry3<LeftUpperArm, LeftShoulder>,
    pub left_elbow_to_upper_arm: Isometry3<LeftElbow, LeftUpperArm>,
    pub left_forearm_to_elbow: Isometry3<LeftForearm, LeftElbow>,
    pub left_wrist_to_forearm: Isometry3<LeftWrist, LeftForearm>,
    pub right_shoulder_to_robot: Isometry3<RightShoulder, Robot>,
    pub right_shoulder_cap_to_robot: Isometry3<RightShoulderCap, Robot>,
    pub right_shoulder_cap_front_to_left_shoulder_cap:
        Isometry3<RightShoulderCapFront, RightShoulderCap>,
    pub right_shoulder_cap_back_to_left_shoulder_cap:
        Isometry3<RightShoulderCapBack, RightShoulderCap>,
    pub right_upper_arm_to_shoulder: Isometry3<RightUpperArm, RightShoulder>,
    pub right_elbow_to_upper_arm: Isometry3<RightElbow, RightUpperArm>,
    pub right_forearm_to_elbow: Isometry3<RightForearm, RightElbow>,
    pub right_wrist_to_forearm: Isometry3<RightWrist, RightForearm>,
    pub left_pelvis_to_robot: Isometry3<LeftPelvis, Robot>,
    pub left_hip_to_pelvis: Isometry3<LeftHip, LeftPelvis>,
    pub left_thigh_to_hip: Isometry3<LeftThigh, LeftHip>,
    pub left_tibia_to_thigh: Isometry3<LeftTibia, LeftThigh>,
    pub left_ankle_to_tibia: Isometry3<LeftAnkle, LeftTibia>,
    pub left_foot_to_ankle: Isometry3<LeftFoot, LeftAnkle>,
    pub left_sole_to_foot: Isometry3<LeftSole, LeftFoot>,
    pub left_toe_to_left_sole: Isometry3<LeftToe, LeftSole>,
    pub right_pelvis_to_robot: Isometry3<RightPelvis, Robot>,
    pub right_hip_to_pelvis: Isometry3<RightHip, RightPelvis>,
    pub right_thigh_to_hip: Isometry3<RightThigh, RightHip>,
    pub right_tibia_to_thigh: Isometry3<RightTibia, RightThigh>,
    pub right_ankle_to_tibia: Isometry3<RightAnkle, RightTibia>,
    pub right_foot_to_ankle: Isometry3<RightFoot, RightAnkle>,
    pub right_sole_to_foot: Isometry3<RightSole, RightFoot>,
    pub right_toe_to_right_sole: Isometry3<RightToe, RightSole>,
}

impl Kinematics {
    #[must_use]
    /// Transform from `S1` to `S2`.
    pub fn transform<T, S1, S2>(&self, x: &InSpace<T, S1>) -> InSpace<T, S2>
    where
        S1: Space + SpaceOver<T>,
        S2: Space + SpaceOver<T>,
        Self: Transform<T, T, S1, S2>,
    {
        Transform::transform(self, x)
    }

    #[must_use]
    /// Get the isometry from `S1` to `S2`.
    pub fn isometry<S1, S2>(&self) -> Isometry3<S1, S2>
    where
        S1: Space + SpaceOver<na::Isometry3<f32>>,
        S2: Space + SpaceOver<na::Isometry3<f32>>,
        Self: Transform<na::Isometry3<f32>, na::Isometry3<f32>, S1, S2>,
    {
        self.transform(&InSpace::new(na::Isometry3::identity()))
            .inner
            .into()
    }

    #[must_use]
    /// Get the vector from `S1` to `S2`.
    pub fn vector<S1, S2>(&self) -> Vector3<S1>
    where
        S1: Space + SpaceOver<na::Point3<f32>> + SpaceOver<na::Vector3<f32>>,
        S2: Space + SpaceOver<na::Point3<f32>> + SpaceOver<na::Vector3<f32>>,
        Self: Transform<na::Point3<f32>, na::Point3<f32>, S2, S1>,
    {
        self.transform(&spatial::point3!(S2)).map(|x| x.coords)
    }

    #[must_use]
    pub fn robot_to_ground(
        &self,
        orientation: na::UnitQuaternion<f32>,
    ) -> (Isometry3<Robot, Ground>, na::UnitQuaternion<f32>) {
        let (roll, pitch, yaw) = orientation.euler_angles();
        let residual = na::UnitQuaternion::from_euler_angles(0., 0., yaw);

        let robot_to_ground: Isometry3<Robot, Ground> = na::Isometry3::from_parts(
            na::Translation3::default(),
            na::UnitQuaternion::from_euler_angles(roll, pitch, 0.),
        )
        .into();

        let contact_points: [Point3<Ground>; 6] = [
            self.transform(&spatial::point3!(LeftSole)),
            self.transform(&spatial::point3!(RightSole)),
            self.transform(&spatial::point3!(LeftWrist)),
            self.transform(&spatial::point3!(RightWrist)),
            self.transform(&spatial::point3!(Robot, 0.05, 0., 0.)),
            self.transform(&spatial::point3!(Robot, -0.05, 0., 0.)),
        ]
        .map(|p| robot_to_ground.transform(&p));

        let height = contact_points
            .into_iter()
            .map(|p| -p.inner.coords.z)
            .max_by(f32::total_cmp)
            .unwrap();

        let robot_to_ground = robot_to_ground.map(|x| na::Translation3::new(0., 0., height) * x);

        (robot_to_ground, residual)
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
    pub fn chest_to_torso() -> Isometry3<Chest, Torso> {
        na::Isometry3::from(TORSO_TO_CHEST).into()
    }

    #[must_use]
    pub fn chest_to_chest_left() -> Isometry3<Chest, ChestLeft> {
        na::Isometry3::from(CHEST_TO_CHEST_LEFT * -1.00).into()
    }

    #[must_use]
    pub fn chest_to_chest_right() -> Isometry3<Chest, ChestRight> {
        na::Isometry3::from(CHEST_TO_CHEST_RIGHT * -1.00).into()
    }

    #[must_use]
    pub fn chest_to_chest_center_left() -> Isometry3<Chest, ChestCenterLeft> {
        na::Isometry3::from(CHEST_TO_CHEST_CENTER_LEFT * -1.00).into()
    }

    #[must_use]
    pub fn chest_to_chest_center_right() -> Isometry3<Chest, ChestCenterRight> {
        na::Isometry3::from(CHEST_TO_CHEST_CENTER_RIGHT * -1.00).into()
    }

    #[must_use]
    pub fn torso_to_robot() -> Isometry3<Torso, Robot> {
        na::Isometry3::from(ROBOT_TO_TORSO).into()
    }

    #[must_use]
    pub fn left_shoulder_to_robot(left_shoulder_pitch: f32) -> Isometry3<LeftShoulder, Robot> {
        na::Isometry3::new(
            ROBOT_TO_LEFT_SHOULDER,
            na::Vector3::y() * left_shoulder_pitch,
        )
        .into()
    }

    #[must_use]
    pub fn left_shoulder_cap_to_robot() -> Isometry3<LeftShoulderCap, Robot> {
        na::Isometry3::from(ROBOT_TO_LEFT_SHOULDER_CAP).into()
    }

    #[must_use]
    pub fn left_shoulder_cap_front_to_left_shoulder_cap(
    ) -> Isometry3<LeftShoulderCapFront, LeftShoulderCap> {
        na::Isometry3::from(SHOULDER_CAP_TO_SHOULDER_CAP_FRONT).into()
    }

    #[must_use]
    pub fn left_shoulder_cap_back_to_left_shoulder_cap(
    ) -> Isometry3<LeftShoulderCapBack, LeftShoulderCap> {
        na::Isometry3::from(SHOULDER_CAP_TO_SHOULDER_CAP_BACK).into()
    }

    #[must_use]
    pub fn left_upper_arm_to_shoulder(
        left_shoulder_roll: f32,
    ) -> Isometry3<LeftUpperArm, LeftShoulder> {
        na::Isometry3::rotation(na::Vector3::z() * left_shoulder_roll).into()
    }

    #[must_use]
    pub fn left_elbow_to_upper_arm(left_elbow_yaw: f32) -> Isometry3<LeftElbow, LeftUpperArm> {
        na::Isometry3::new(
            LEFT_SHOULDER_TO_LEFT_ELBOW,
            na::Vector3::x() * left_elbow_yaw,
        )
        .into()
    }

    #[must_use]
    pub fn left_forearm_to_elbow(left_elbow_roll: f32) -> Isometry3<LeftForearm, LeftElbow> {
        na::Isometry3::rotation(na::Vector3::z() * left_elbow_roll).into()
    }

    #[must_use]
    pub fn left_wrist_to_forearm(left_wrist_yaw: f32) -> Isometry3<LeftWrist, LeftForearm> {
        na::Isometry3::new(ELBOW_TO_WRIST, na::Vector3::x() * left_wrist_yaw).into()
    }

    #[must_use]
    pub fn right_shoulder_to_robot(right_shoulder_pitch: f32) -> Isometry3<RightShoulder, Robot> {
        na::Isometry3::new(
            ROBOT_TO_RIGHT_SHOULDER,
            na::Vector3::y() * right_shoulder_pitch,
        )
        .into()
    }

    #[must_use]
    pub fn right_shoulder_cap_to_robot() -> Isometry3<RightShoulderCap, Robot> {
        na::Isometry3::from(ROBOT_TO_RIGHT_SHOULDER_CAP).into()
    }

    #[must_use]
    pub fn right_shoulder_cap_front_to_right_shoulder_cap(
    ) -> Isometry3<RightShoulderCapFront, RightShoulderCap> {
        na::Isometry3::from(SHOULDER_CAP_TO_SHOULDER_CAP_FRONT).into()
    }

    #[must_use]
    pub fn right_shoulder_cap_back_to_right_shoulder_cap(
    ) -> Isometry3<RightShoulderCapBack, RightShoulderCap> {
        na::Isometry3::from(SHOULDER_CAP_TO_SHOULDER_CAP_BACK).into()
    }

    #[must_use]
    pub fn right_upper_arm_to_shoulder(
        right_shoulder_roll: f32,
    ) -> Isometry3<RightUpperArm, RightShoulder> {
        na::Isometry3::rotation(na::Vector3::z() * right_shoulder_roll).into()
    }

    #[must_use]
    pub fn right_elbow_to_upper_arm(right_elbow_yaw: f32) -> Isometry3<RightElbow, RightUpperArm> {
        na::Isometry3::new(
            RIGHT_SHOULDER_TO_RIGHT_ELBOW,
            na::Vector3::x() * right_elbow_yaw,
        )
        .into()
    }

    #[must_use]
    pub fn right_forearm_to_elbow(right_elbow_roll: f32) -> Isometry3<RightForearm, RightElbow> {
        na::Isometry3::rotation(na::Vector3::z() * right_elbow_roll).into()
    }

    #[must_use]
    pub fn right_wrist_to_forearm(right_wrist_yaw: f32) -> Isometry3<RightWrist, RightForearm> {
        na::Isometry3::new(ELBOW_TO_WRIST, na::Vector3::x() * right_wrist_yaw).into()
    }

    #[must_use]
    pub fn left_pelvis_to_robot(left_hip_yaw_pitch: f32) -> Isometry3<LeftPelvis, Robot> {
        na::Isometry3::new(
            ROBOT_TO_LEFT_PELVIS,
            na::Vector3::new(0., FRAC_1_SQRT_2, -FRAC_1_SQRT_2) * left_hip_yaw_pitch,
        )
        .into()
    }

    #[must_use]
    pub fn left_hip_to_pelvis(left_hip_roll: f32) -> Isometry3<LeftHip, LeftPelvis> {
        na::Isometry3::rotation(na::Vector3::x() * left_hip_roll).into()
    }

    #[must_use]
    pub fn left_thigh_to_hip(left_hip_pitch: f32) -> Isometry3<LeftThigh, LeftHip> {
        na::Isometry3::rotation(na::Vector3::y() * left_hip_pitch).into()
    }

    #[must_use]
    pub fn left_tibia_to_thigh(left_knee_pitch: f32) -> Isometry3<LeftTibia, LeftThigh> {
        na::Isometry3::new(HIP_TO_KNEE, na::Vector3::y() * left_knee_pitch).into()
    }

    #[must_use]
    pub fn left_ankle_to_tibia(left_ankle_pitch: f32) -> Isometry3<LeftAnkle, LeftTibia> {
        na::Isometry3::new(KNEE_TO_ANKLE, na::Vector3::y() * left_ankle_pitch).into()
    }

    #[must_use]
    pub fn left_foot_to_ankle(left_ankle_roll: f32) -> Isometry3<LeftFoot, LeftAnkle> {
        na::Isometry3::rotation(na::Vector3::x() * left_ankle_roll).into()
    }

    #[must_use]
    pub fn left_sole_to_foot() -> Isometry3<LeftSole, LeftFoot> {
        na::Isometry3::from(ANKLE_TO_SOLE).into()
    }

    #[must_use]
    pub fn left_toe_to_left_sole() -> Isometry3<LeftToe, LeftSole> {
        na::Isometry3::from(SOLE_TO_TOE).into()
    }

    #[must_use]
    pub fn right_pelvis_to_robot(left_hip_yaw_pitch: f32) -> Isometry3<RightPelvis, Robot> {
        na::Isometry3::new(
            ROBOT_TO_RIGHT_PELVIS,
            na::Vector3::new(0., FRAC_1_SQRT_2, FRAC_1_SQRT_2) * left_hip_yaw_pitch,
        )
        .into()
    }

    #[must_use]
    pub fn right_hip_to_pelvis(right_hip_roll: f32) -> Isometry3<RightHip, RightPelvis> {
        na::Isometry3::rotation(na::Vector3::x() * right_hip_roll).into()
    }

    #[must_use]
    pub fn right_thigh_to_hip(right_hip_pitch: f32) -> Isometry3<RightThigh, RightHip> {
        na::Isometry3::rotation(na::Vector3::y() * right_hip_pitch).into()
    }

    #[must_use]
    pub fn right_tibia_to_thigh(right_knee_pitch: f32) -> Isometry3<RightTibia, RightThigh> {
        na::Isometry3::new(HIP_TO_KNEE, na::Vector3::y() * right_knee_pitch).into()
    }

    #[must_use]
    pub fn right_ankle_to_tibia(right_ankle_pitch: f32) -> Isometry3<RightAnkle, RightTibia> {
        na::Isometry3::new(KNEE_TO_ANKLE, na::Vector3::y() * right_ankle_pitch).into()
    }

    #[must_use]
    pub fn right_foot_to_ankle(right_ankle_roll: f32) -> Isometry3<RightFoot, RightAnkle> {
        na::Isometry3::rotation(na::Vector3::x() * right_ankle_roll).into()
    }

    #[must_use]
    pub fn right_sole_to_foot() -> Isometry3<RightSole, RightFoot> {
        na::Isometry3::from(ANKLE_TO_SOLE).into()
    }

    #[must_use]
    pub fn right_toe_to_right_sole() -> Isometry3<RightToe, RightSole> {
        na::Isometry3::from(SOLE_TO_TOE).into()
    }
}

impl From<&JointArray<f32>> for Kinematics {
    fn from(joints: &JointArray<f32>) -> Self {
        Self {
            head_to_neck: Self::head_to_neck(joints.head_pitch),
            neck_to_robot: Self::neck_to_robot(joints.head_yaw),
            torso_to_robot: Self::torso_to_robot(),
            chest_to_torso: Self::chest_to_torso(),
            chest_to_chest_left: Self::chest_to_chest_left(),
            chest_to_chest_right: Self::chest_to_chest_right(),
            chest_to_chest_center_left: Self::chest_to_chest_center_left(),
            chest_to_chest_center_right: Self::chest_to_chest_center_right(),
            left_shoulder_to_robot: Self::left_shoulder_to_robot(joints.left_shoulder_pitch),
            left_shoulder_cap_to_robot: Self::left_shoulder_cap_to_robot(),
            left_shoulder_cap_front_to_left_shoulder_cap:
                Self::left_shoulder_cap_front_to_left_shoulder_cap(),
            left_shoulder_cap_back_to_left_shoulder_cap:
                Self::left_shoulder_cap_back_to_left_shoulder_cap(),
            left_upper_arm_to_shoulder: Self::left_upper_arm_to_shoulder(joints.left_shoulder_roll),
            left_elbow_to_upper_arm: Self::left_elbow_to_upper_arm(joints.left_elbow_yaw),
            left_forearm_to_elbow: Self::left_forearm_to_elbow(joints.left_elbow_roll),
            left_wrist_to_forearm: Self::left_wrist_to_forearm(joints.left_wrist_yaw),
            right_shoulder_to_robot: Self::right_shoulder_to_robot(joints.right_shoulder_pitch),
            right_shoulder_cap_to_robot: Self::right_shoulder_cap_to_robot(),
            right_shoulder_cap_front_to_left_shoulder_cap:
                Self::right_shoulder_cap_front_to_right_shoulder_cap(),
            right_shoulder_cap_back_to_left_shoulder_cap:
                Self::right_shoulder_cap_back_to_right_shoulder_cap(),
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
            left_toe_to_left_sole: Self::left_toe_to_left_sole(),
            right_pelvis_to_robot: Self::right_pelvis_to_robot(joints.left_hip_yaw_pitch),
            right_hip_to_pelvis: Self::right_hip_to_pelvis(joints.right_hip_roll),
            right_thigh_to_hip: Self::right_thigh_to_hip(joints.right_hip_pitch),
            right_tibia_to_thigh: Self::right_tibia_to_thigh(joints.right_knee_pitch),
            right_ankle_to_tibia: Self::right_ankle_to_tibia(joints.right_ankle_pitch),
            right_foot_to_ankle: Self::right_foot_to_ankle(joints.right_ankle_roll),
            right_sole_to_foot: Self::right_sole_to_foot(),
            right_toe_to_right_sole: Self::right_toe_to_right_sole(),
        }
    }
}

impl Default for Kinematics {
    fn default() -> Self {
        Self::from(&JointArray::default())
    }
}
