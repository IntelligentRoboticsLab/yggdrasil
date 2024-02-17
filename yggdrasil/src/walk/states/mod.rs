pub mod idle;
pub mod walking;

use enum_dispatch::enum_dispatch;
use nidhogg::types::ForceSensitiveResistors;
use std::time::Duration;

use crate::kinematics::FootOffset;

use super::{engine::WalkCommand, FilteredGyroscope, WalkingEngineConfig};

pub struct WalkContext<'a> {
    pub walk_command: WalkCommand,
    pub dt: Duration,
    pub config: &'a WalkingEngineConfig,
    pub filtered_gyro: &'a FilteredGyroscope,
    pub fsr: &'a ForceSensitiveResistors,
}

#[enum_dispatch]
pub trait WalkState {
    fn next_state(self, context: WalkContext) -> WalkStateKind;
    fn get_foot_offsets(&self) -> (FootOffset, FootOffset);
}

#[derive(Debug, Clone)]
#[enum_dispatch(WalkState)]
pub enum WalkStateKind {
    Idle(idle::IdleState),
    Walking(walking::WalkingState),
}
