use std::{
    ops::Div,
    time::{Duration, Instant},
};

use nidhogg::types::{ArmJoints, HeadJoints, LegJoints};

pub struct JointInterpolator {
    duration: Duration,
    start: Instant,
}

impl JointInterpolator {
    pub fn new(duration: Duration) -> Self {
        JointInterpolator {
            duration,
            start: Instant::now(),
        }
    }
}

trait Joint: Sized + std::ops::Add<Output = Self> + std::ops::Mul<f32, Output = Self> {}

impl Joint for HeadJoints<f32> {}
impl Joint for ArmJoints<f32> {}
impl Joint for LegJoints<f32> {}

impl JointInterpolator {
    pub fn interpolated_positions<T: Joint>(
        &self,
        start_joint_positions: T,
        end_joint_positions: T,
    ) -> T {
        let weight = self
            .start
            .elapsed()
            .as_secs_f32()
            .div(self.duration.as_secs_f32())
            .max(1.0);

        start_joint_positions * (1.0 - weight) + end_joint_positions * weight
    }
}
