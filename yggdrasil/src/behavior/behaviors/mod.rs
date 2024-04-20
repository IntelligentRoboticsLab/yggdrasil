//! All the behaviors that the robot can execute.

mod catchfall;
mod energy_efficient_stand;
mod initial;
mod observe;
mod penalized;
mod standup;
mod startup;
mod unstiff;
mod walk;

pub use catchfall::CatchFall;
pub use energy_efficient_stand::EnergyEfficientStand;
pub use initial::Initial;
pub use observe::{Observe, ObserveBehaviorConfig};
pub use penalized::Penalized;
pub use standup::Standup;
pub use startup::StartUp;
pub use unstiff::Unstiff;
pub use walk::Walk;
