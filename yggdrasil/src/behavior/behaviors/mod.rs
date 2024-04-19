//! All the behaviors that the robot can execute.

mod initial;
mod observe;
mod penalized;
mod startup;
mod unstiff;
mod walk;
mod energy_efficient_stand;

pub use initial::Initial;
pub use observe::{Observe, ObserveBehaviorConfig};
pub use penalized::Penalized;
pub use startup::StartUp;
pub use unstiff::Unstiff;
pub use walk::Walk;
pub use energy_efficient_stand::EnergyEfficientStand;
