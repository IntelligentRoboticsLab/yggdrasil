use nalgebra::{Isometry3, Translation3, UnitQuaternion, Vector3};
use spatial::types::Pose3;

use crate::kinematics::{
    prelude::{ROBOT_TO_LEFT_PELVIS, ROBOT_TO_RIGHT_PELVIS},
    spaces::{Ground, LeftSole, RightSole, Robot},
    Kinematics,
};

use super::{step::Step, Side};

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

        // compute the pose of the feet in robot frame
        let left_foot = kinematics.isometry::<LeftSole, Robot>().inner * offset;
        let right_foot = kinematics.isometry::<RightSole, Robot>().inner * offset;

        // println!("after: LEFT {:?}", left_foot.translation);
        // println!("after: RIGHT {:?}", right_foot.translation);
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
}
