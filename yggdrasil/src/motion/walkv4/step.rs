use std::time::Duration;

use super::Side;

#[derive(Debug, Clone)]
pub struct Step {
    pub forward: f32,
    pub lateral: f32,
    pub turn: f32,
    pub duration: Duration,
    pub apex: f32,
    pub swing_foot: Side,
}

impl Step {
    pub fn new(
        forward: f32,
        lateral: f32,
        turn: f32,
        duration: Duration,
        apex: f32,
        swing_foot: Side,
    ) -> Self {
        Self {
            forward,
            lateral,
            turn,
            duration,
            apex,
            swing_foot,
        }
    }
}
