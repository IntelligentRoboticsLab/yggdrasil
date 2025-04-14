use nalgebra::{Isometry3, Translation3, UnitQuaternion, Vector2, Vector3};
use spatial::types::Pose3;

use super::{Side, step::Step};
use crate::kinematics::{
    FootOffset, Kinematics,
    prelude::{ROBOT_TO_LEFT_PELVIS, ROBOT_TO_RIGHT_PELVIS},
    spaces::{Ground, LeftSole, RightSole, Robot},
};

/// Position of the left and right foot of the robot, relative to the ground.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FootPositions {
    pub left: Pose3<Ground>,
    pub right: Pose3<Ground>,
}

impl Default for FootPositions {
    fn default() -> Self {
        Self::from_target(Side::Left, &Step::default())
    }
}

impl FootPositions {
    #[must_use]
    pub fn new(left: Pose3<Ground>, right: Pose3<Ground>) -> Self {
        Self { left, right }
    }

    #[must_use]
    pub fn from_kinematics(swing_side: Side, kinematics: &Kinematics, torso_offset: f32) -> Self {
        let hip_height = match swing_side {
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

    #[must_use]
    pub fn from_target(swing_side: Side, step: &Step) -> Self {
        let (support_offset, swing_offset) = match swing_side {
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

        let (left, right) = match swing_side {
            Side::Left => (swing_sole, support_sole),
            Side::Right => (support_sole, swing_sole),
        };

        Self { left, right }
    }

    #[must_use]
    pub fn to_offsets(&self, hip_height: f32) -> (FootOffset, FootOffset) {
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

        (left, right)
    }

    /// Compute the distance travelled by the swing foot in the ground plane.
    #[must_use]
    pub fn swing_translation(&self, swing_side: Side, target: &FootPositions) -> Vector2<f32> {
        match swing_side {
            // this equals: (self.right - self.left) + (target.left - target.right)
            Side::Left => {
                (self.right.translation * self.left.translation.inverse())
                    * (target.left.translation * target.right.translation.inverse())
            }
            // this equals: (self.left - self.right) + (target.right - target.left)
            Side::Right => {
                (self.left.translation * self.right.translation.inverse())
                    * (target.right.translation * target.left.translation.inverse())
            }
        }
        .vector
        .xy()
    }

    /// Compute the angle between the current foot positions and the provided target.
    #[must_use]
    pub fn turn_amount(&self, swing_side: Side, target: &FootPositions) -> f32 {
        match &swing_side {
            Side::Left => target.left.rotation.angle_to(&self.left.rotation),
            Side::Right => target.right.rotation.angle_to(&self.right.rotation),
        }
    }
}
