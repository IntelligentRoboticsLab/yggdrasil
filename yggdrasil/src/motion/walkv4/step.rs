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
