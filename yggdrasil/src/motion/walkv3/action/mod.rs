use std::time::Duration;

use enum_dispatch::enum_dispatch;

use crate::{core::debug::DebugContext, kinematics::RobotKinematics, nao::manager::NaoManager};

mod sit;
mod stand;
mod walk;

pub use sit::*;
pub use stand::*;
pub use walk::*;

pub struct UpdateContext {
    pub kinematics: RobotKinematics,
    pub delta_time: Duration,
}

#[enum_dispatch]
pub trait WalkAction {
    fn update(&mut self, context: &UpdateContext);

    fn apply(&self, nao: &mut NaoManager, ctx: &DebugContext);
}

#[enum_dispatch(WalkAction)]
pub enum Action {
    Stand(stand::Standing),
    Sit(sit::Sitting),
    Walk(walk::Walking),
}
