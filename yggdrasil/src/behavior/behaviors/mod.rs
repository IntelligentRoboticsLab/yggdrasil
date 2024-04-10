//! All the behaviors that the robot can execute.

mod startup;
mod unstiff;
mod initial;
mod observe;
mod penalized;

pub use startup::StartUp;
pub use unstiff::Unstiff;
pub use initial::Initial;
pub use observe::{Observe, ObserveBehaviorConfig};
pub use penalized::Penalized;
