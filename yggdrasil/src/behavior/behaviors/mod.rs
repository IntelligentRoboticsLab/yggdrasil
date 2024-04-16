//! All the behaviors that the robot can execute.

mod initial;
mod observe;
mod passive;
mod penalized;
mod standup;

pub use initial::Initial;
pub use observe::{Observe, ObserveBehaviorConfig};
pub use passive::Passive;
pub use penalized::Penalized;
pub use standup::Standup;
