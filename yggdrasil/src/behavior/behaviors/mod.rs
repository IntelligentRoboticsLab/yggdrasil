//! All the behaviors that the robot can execute.

mod initial;
mod observe;
mod penalized;
mod startup;
mod unstiff;

pub use initial::Initial;
pub use observe::{Observe, ObserveBehaviorConfig};
pub use penalized::Penalized;
pub use startup::StartUp;
pub use unstiff::Unstiff;
