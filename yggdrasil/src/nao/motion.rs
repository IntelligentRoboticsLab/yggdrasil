use std::time::{Duration, Instant};

use nidhogg::types::JointArray;
use tyr::prelude::*;

pub struct Motion {
    pub start: JointArray<f32>,
    pub target: JointArray<f32>,
    pub time: Duration,
    pub initial: Instant
}

wrap!(JointPositions, JointArray<f32>);
wrap!(JointStiffness, JointArray<f32>);
