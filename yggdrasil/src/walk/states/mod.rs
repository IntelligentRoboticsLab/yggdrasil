pub mod idle;
pub mod walking;

use enum_dispatch::enum_dispatch;
use nidhogg::types::{ForceSensitiveResistors, Vector2};
use std::time::Duration;

use crate::filter::imu::IMUValues;

use super::engine::WalkCommand;

pub struct WalkContext<'a> {
    pub walk_command: WalkCommand,
    pub dt: Duration,
    pub filtered_gyro: Vector2<f32>,
    pub fsr: ForceSensitiveResistors,
    pub control_message: &'a mut nidhogg::NaoControlMessage,
}

#[enum_dispatch]
pub trait WalkState {
    fn next_state<'a>(&self, context: &'a mut WalkContext) -> WalkStateKind;
}

#[derive(Debug)]
#[enum_dispatch(WalkState)]
pub enum WalkStateKind {
    Idle(idle::IdleState),
    Walking(walking::WalkingState),
}

impl Default for WalkStateKind {
    fn default() -> Self {
        Self::Idle(idle::IdleState::default())
    }
}
