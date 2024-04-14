//! All the behaviors that the robot can execute.

mod initial;
mod observe;
mod penalized;
mod startup;
mod test;
mod unstiff;
mod walk;

pub use initial::Initial;
pub use observe::{Observe, ObserveBehaviorConfig};
pub use penalized::Penalized;
pub use startup::StartUp;
pub use test::Test;
pub use unstiff::Unstiff;
pub use walk::Walk;
