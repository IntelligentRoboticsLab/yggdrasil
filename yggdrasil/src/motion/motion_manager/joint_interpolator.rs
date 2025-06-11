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

// TODO: Move to nidhogg.
pub trait Joint<T>:
    Sized
    + std::ops::Add<Output = Self>
    + std::ops::Add<T, Output = Self>
    + std::ops::Sub<Output = Self>
    + std::ops::Sub<T, Output = Self>
    + std::ops::Mul<Output = Self>
    + std::ops::Mul<T, Output = Self>
    + std::ops::Div<Output = Self>
    + std::ops::Div<T, Output = Self>
{
}

// TODO: Move to nidhogg.
impl<T> Joint<T> for HeadJoints<T> where
    T: Clone
        + std::ops::Add<Output = T>
        + std::ops::Add<T, Output = T>
        + std::ops::Sub<Output = T>
        + std::ops::Sub<T, Output = T>
        + std::ops::Mul<Output = T>
        + std::ops::Mul<T, Output = T>
        + std::ops::Div<Output = T>
        + std::ops::Div<T, Output = T>
{
}

// TODO: Move to nidhogg.
impl<T> Joint<T> for ArmJoints<T> where
    T: Clone
        + std::ops::Add<Output = T>
        + std::ops::Add<T, Output = T>
        + std::ops::Sub<Output = T>
        + std::ops::Sub<T, Output = T>
        + std::ops::Mul<Output = T>
        + std::ops::Mul<T, Output = T>
        + std::ops::Div<Output = T>
        + std::ops::Div<T, Output = T>
{
}

// TODO: Move to nidhogg.
impl<T> Joint<T> for LegJoints<T> where
    T: Clone
        + std::ops::Add<Output = T>
        + std::ops::Add<T, Output = T>
        + std::ops::Sub<Output = T>
        + std::ops::Sub<T, Output = T>
        + std::ops::Mul<Output = T>
        + std::ops::Mul<T, Output = T>
        + std::ops::Div<Output = T>
        + std::ops::Div<T, Output = T>
{
}

impl JointInterpolator {
    pub fn interpolated_positions<T: Joint<f32>>(
        &self,
        start_joint_positions: T,
        end_joint_positions: T,
    ) -> T {
        let weight = self
            .start
            .elapsed()
            .as_secs_f32()
            .div(self.duration.as_secs_f32())
            .min(1.0);

        start_joint_positions * (1.0 - weight) + end_joint_positions * weight
    }
}
