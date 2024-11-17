use nalgebra::Translation3;
use spatial::{
    types::{Isometry3, Pose3},
    InSpace,
};

use crate::kinematics::{
    spaces::{Ground, LeftPelvis, LeftSole, RightPelvis, RightSole, Robot},
    Kinematics,
};

use super::Side;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FootPositions {
    pub left: Pose3<Ground>,
    pub right: Pose3<Ground>,
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

        let offset = Translation3::new(torso_offset, 0., hip_height);

        // compute the pose of the feet in robot frame
        let left_foot = kinematics.isometry::<LeftSole, Robot>().inner * offset;
        let right_foot = kinematics.isometry::<RightSole, Robot>().inner * offset;

        Self {
            left: Pose3::new(left_foot),
            right: Pose3::new(right_foot),
        }
    }
}
