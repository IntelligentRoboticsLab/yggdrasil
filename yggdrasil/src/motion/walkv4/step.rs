use std::time::Duration;

use super::Side;

#[derive(Debug, Clone)]
pub struct Step {
    pub forward: f32,
    pub left: f32,
    pub turn: f32,
    pub duration: Duration,
    pub swing_foot_height: f32,
    pub swing_foot: Side,
}

impl Default for Step {
    fn default() -> Self {
        Self {
            forward: 0f32,
            left: 0f32,
            turn: 0f32,
            duration: Duration::ZERO,
            swing_foot_height: 0f32,
            swing_foot: Side::default(),
        }
    }
}
