use nalgebra::Isometry3;
use nidhogg::types::LegJoints;

use crate::kinematics::{self, RobotKinematics};

pub struct Feet {
    pub support: Isometry3<f32>,
    pub swing: Isometry3<f32>,
}

impl Feet {
    pub fn from_joints(kinematics: &RobotKinematics) -> Self {
        let left = kinematics.left_sole_to_robot;
    }
}
