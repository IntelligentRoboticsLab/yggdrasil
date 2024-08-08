use nalgebra::{Isometry3, Translation, UnitQuaternion, Vector3};

use crate::{kinematics::RobotKinematics, motion::walk::engine::Side};

use super::step::StepRequest;

const LEFT_FOOT_OFFSET: Vector3<f32> = Vector3::new(0.00, -0.052, 0.0);
const RIGHT_FOOT_OFFSET: Vector3<f32> = Vector3::new(0.00, 0.052, 0.0);

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Feet {
    pub support: Isometry3<f32>,
    pub swing: Isometry3<f32>,
}

impl Feet {
    /// Get the current feet positions from the leg joints
    pub fn from_joints(kinematics: &RobotKinematics, swing_side: Side) -> Self {
        let robot_to_walk = Isometry3::from_parts(
            Translation::from(Vector3::new(0.0, 0.0, 0.225)),
            UnitQuaternion::new(Vector3::new(0.0, 0.055, 0.0)),
        );
        let (swing, support) = match swing_side {
            Side::Left => (
                kinematics.left_sole_to_robot,
                kinematics.right_sole_to_robot,
            ),
            Side::Right => (
                kinematics.right_sole_to_robot,
                kinematics.left_sole_to_robot,
            ),
        };

        let (swing_base_offset, support_base_offset) = match swing_side {
            Side::Left => (LEFT_FOOT_OFFSET, RIGHT_FOOT_OFFSET),
            Side::Right => (RIGHT_FOOT_OFFSET, LEFT_FOOT_OFFSET),
        };

        let swing = robot_to_walk
            * swing
            * Isometry3::from_parts(
                Translation::from(swing_base_offset),
                UnitQuaternion::identity(),
            );
        let support = robot_to_walk
            * support
            * Isometry3::from_parts(
                Translation::from(support_base_offset),
                UnitQuaternion::identity(),
            );

        Self { swing, support }
    }

    /// Get the current feet positions from the leg joints
    pub fn from_request(step: StepRequest, swing_side: Side) -> Self {
        let swing = Vector3::new(step.forward / 2.0, step.left / 2.0, 0.0);
        let support = Vector3::new(-step.forward / 2.0, -step.left / 2.0, 0.0);

        let swing = Isometry3::from_parts(
            Translation::from(swing),
            UnitQuaternion::new(Vector3::new(0.0, 0.0, step.turn / 2.0)),
        );

        let support = Isometry3::from_parts(
            Translation::from(support),
            UnitQuaternion::new(Vector3::new(0.0, 0.0, -step.turn / 2.0)),
        );

        Self { support, swing }
    }
}
