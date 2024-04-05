//! All the behaviors that the robot can execute.

mod initial;
mod observe;
mod passive;

pub use initial::Initial;
pub use observe::{Observe, ObserveBehaviorConfig};
pub use passive::Passive;
