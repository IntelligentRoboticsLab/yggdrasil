use nalgebra::{Isometry3, Translation3, UnitQuaternion, Vector3};
use spatial::types::Pose3;

use super::{step::Step, Side};
use crate::{
    kinematics::{
        prelude::{ROBOT_TO_LEFT_PELVIS, ROBOT_TO_RIGHT_PELVIS},
        spaces::{Ground, LeftSole, RightSole, Robot},
        FootOffset, Kinematics,
    },
    motion::walk::engine::FootOffsets,
};
use bevy::prelude::*;

/// Position of the left and right foot of the robot, relative to the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FootPositions {
    pub left: Pose3<Ground>,
    pub right: Pose3<Ground>,
}

impl Default for FootPositions {
    fn default() -> Self {
        Self::from_target(&Step::default())
    }
}

impl FootPositions {
    pub fn new(left: Pose3<Ground>, right: Pose3<Ground>) -> Self {
        Self { left, right }
    }

    pub fn from_kinematics(support_foot: Side, kinematics: &Kinematics, torso_offset: f32) -> Self {
        let hip_height = match support_foot {
            Side::Left => kinematics.left_hip_height(),
            Side::Right => kinematics.right_hip_height(),
        };

        // println!(
        //     "[{:?}] from kinematics hip height: {}",
        //     support_foot, hip_height
        // );

        // println!(
        //     "before: LEFT {:?}",
        //     kinematics.isometry::<LeftSole, Robot>().inner.translation
        // );
        // println!(
        //     "before: RIGHT {:?}",
        //     kinematics.isometry::<RightSole, Robot>().inner.translation
        // );

        let offset = Translation3::new(torso_offset, 0., hip_height);
        info!("robot_to_walk: {:?}", offset);

        // compute the pose of the feet in robot frame
        let left_foot = kinematics.isometry::<LeftSole, Robot>().inner * offset;
        let right_foot = kinematics.isometry::<RightSole, Robot>().inner * offset;

        info!(
            "left: {:?}, {:?}",
            kinematics.isometry::<LeftSole, Robot>().inner.translation,
            kinematics
                .isometry::<LeftSole, Robot>()
                .inner
                .rotation
                .euler_angles()
        );
        info!(
            "left: {:?}, {:?}",
            left_foot.translation,
            left_foot.rotation.euler_angles()
        );
        info!(
            "right: {:?}, {:?}",
            kinematics.isometry::<RightSole, Robot>().inner.translation,
            kinematics
                .isometry::<RightSole, Robot>()
                .inner
                .rotation
                .euler_angles()
        );
        info!(
            "right: {:?}, {:?}",
            right_foot.translation,
            right_foot.rotation.euler_angles()
        );
        Self {
            left: Pose3::new(left_foot),
            right: Pose3::new(right_foot),
        }
    }

    pub fn from_target(step: &Step) -> Self {
        let (support_offset, swing_offset) = match step.swing_foot {
            Side::Left => (ROBOT_TO_RIGHT_PELVIS, ROBOT_TO_LEFT_PELVIS),
            Side::Right => (ROBOT_TO_LEFT_PELVIS, ROBOT_TO_RIGHT_PELVIS),
        };

        let support_sole = Pose3::new(Isometry3::from_parts(
            Translation3::new(-step.forward / 2., -step.left / 2.0 + support_offset.y, 0.0),
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), -step.turn / 2.),
        ));

        let swing_sole = Pose3::new(Isometry3::from_parts(
            Translation3::new(step.forward / 2., step.left / 2.0 + swing_offset.y, 0.0),
            UnitQuaternion::from_axis_angle(&Vector3::z_axis(), step.turn / 2.),
        ));

        let (left, right) = match step.swing_foot {
            Side::Left => (swing_sole, support_sole),
            Side::Right => (support_sole, swing_sole),
        };

        Self { left, right }
    }

    // TODO: Re-implement the turning
    // TODO: Get rid of FootOffsets and use [`FootPositions`]
    pub fn to_offsets(&self, hip_height: f32) -> FootOffsets {
        info!(
            "requesting left turn of: {:.4}",
            self.left.rotation.euler_angles().2
        );
        info!(
            "requesting right turn of: {:.4}",
            -self.right.rotation.euler_angles().2
        );
        let left = FootOffset {
            forward: self.left.translation.x,
            left: self.left.translation.y - ROBOT_TO_LEFT_PELVIS.y,
            turn: self.left.rotation.euler_angles().2,
            lift: self.left.translation.z,
            hip_height,
        };

        let right = FootOffset {
            forward: self.right.translation.x,
            left: self.right.translation.y - ROBOT_TO_RIGHT_PELVIS.y,
            turn: -self.right.rotation.euler_angles().2,
            lift: self.right.translation.z,
            hip_height,
        };

        FootOffsets { left, right }
    }
}
