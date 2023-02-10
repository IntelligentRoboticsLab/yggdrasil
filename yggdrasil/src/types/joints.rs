use std::ops::{Add, Mul};

use nidhogg::types::JointArray;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HeadJoints {
    pub yaw: f32,
    pub pitch: f32,
}

impl HeadJoints {
    pub fn mirrored(self) -> Self {
        Self {
            yaw: -self.yaw,
            pitch: self.pitch,
        }
    }

    pub fn fill(value: f32) -> Self {
        Self {
            yaw: value,
            pitch: value,
        }
    }
}

impl From<JointArray<f32>> for HeadJoints {
    fn from(joints: JointArray<f32>) -> Self {
        Self {
            yaw: joints.head_pitch,
            pitch: joints.head_pitch,
        }
    }
}

impl Mul<f32> for HeadJoints {
    type Output = HeadJoints;

    fn mul(self, scale_factor: f32) -> Self::Output {
        Self::Output {
            yaw: self.yaw * scale_factor,
            pitch: self.pitch * scale_factor,
        }
    }
}

impl Add for HeadJoints {
    type Output = HeadJoints;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            yaw: self.yaw + rhs.yaw,
            pitch: self.pitch + rhs.pitch,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ArmJoints {
    pub shoulder_pitch: f32,
    pub shoulder_roll: f32,
    pub elbow_yaw: f32,
    pub elbow_roll: f32,
    pub wrist_yaw: f32,
    pub hand: f32,
}

impl ArmJoints {
    pub fn mirrored(self) -> Self {
        Self {
            shoulder_pitch: self.shoulder_pitch,
            shoulder_roll: -self.shoulder_roll,
            elbow_yaw: -self.elbow_yaw,
            elbow_roll: -self.elbow_roll,
            wrist_yaw: -self.wrist_yaw,
            hand: self.hand,
        }
    }

    pub fn fill(value: f32) -> Self {
        Self {
            shoulder_pitch: value,
            shoulder_roll: value,
            elbow_yaw: value,
            elbow_roll: value,
            wrist_yaw: value,
            hand: value,
        }
    }
}

impl Mul<f32> for ArmJoints {
    type Output = ArmJoints;

    fn mul(self, scale_factor: f32) -> Self::Output {
        Self::Output {
            shoulder_pitch: self.shoulder_pitch * scale_factor,
            shoulder_roll: self.shoulder_roll * scale_factor,
            elbow_yaw: self.elbow_yaw * scale_factor,
            elbow_roll: self.elbow_roll * scale_factor,
            wrist_yaw: self.wrist_yaw * scale_factor,
            hand: self.hand * scale_factor,
        }
    }
}

impl Add for ArmJoints {
    type Output = ArmJoints;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            shoulder_pitch: self.shoulder_pitch + rhs.shoulder_pitch,
            shoulder_roll: self.shoulder_roll + rhs.shoulder_roll,
            elbow_yaw: self.elbow_yaw + rhs.elbow_yaw,
            elbow_roll: self.elbow_roll + rhs.elbow_roll,
            wrist_yaw: self.wrist_yaw + rhs.wrist_yaw,
            hand: self.hand + rhs.hand,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LegJoints {
    pub hip_yaw_pitch: f32,
    pub hip_roll: f32,
    pub hip_pitch: f32,
    pub knee_pitch: f32,
    pub ankle_pitch: f32,
    pub ankle_roll: f32,
}

impl LegJoints {
    pub fn mirrored(self) -> Self {
        Self {
            hip_yaw_pitch: self.hip_yaw_pitch,
            hip_roll: -self.hip_roll,
            hip_pitch: self.hip_pitch,
            knee_pitch: self.knee_pitch,
            ankle_pitch: self.ankle_pitch,
            ankle_roll: -self.ankle_roll,
        }
    }

    pub fn fill(value: f32) -> Self {
        Self {
            hip_yaw_pitch: value,
            hip_roll: value,
            hip_pitch: value,
            knee_pitch: value,
            ankle_pitch: value,
            ankle_roll: value,
        }
    }
}

impl Mul<f32> for LegJoints {
    type Output = LegJoints;

    fn mul(self, scale_factor: f32) -> Self::Output {
        Self::Output {
            hip_yaw_pitch: self.hip_yaw_pitch * scale_factor,
            hip_roll: self.hip_roll * scale_factor,
            hip_pitch: self.hip_pitch * scale_factor,
            knee_pitch: self.knee_pitch * scale_factor,
            ankle_pitch: self.ankle_pitch * scale_factor,
            ankle_roll: self.ankle_roll * scale_factor,
        }
    }
}

impl Add for LegJoints {
    type Output = LegJoints;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            hip_yaw_pitch: self.hip_yaw_pitch + rhs.hip_yaw_pitch,
            hip_roll: self.hip_roll + rhs.hip_roll,
            hip_pitch: self.hip_pitch + rhs.hip_pitch,
            knee_pitch: self.knee_pitch + rhs.knee_pitch,
            ankle_pitch: self.ankle_pitch + rhs.ankle_pitch,
            ankle_roll: self.ankle_roll + rhs.ankle_roll,
        }
    }
}
