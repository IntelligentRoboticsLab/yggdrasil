//! All the behaviors that the robot can execute.

mod fallcatch;
mod initial;
mod observe;
mod penalized;
mod standup;
mod startup;
mod unstiff;
mod walk;

pub use fallcatch::FallCatch;
pub use initial::Initial;
pub use observe::{Observe, ObserveBehaviorConfig};
pub use penalized::Penalized;
pub use standup::Standup;
pub use startup::StartUp;
pub use unstiff::Unstiff;
pub use walk::Walk;
