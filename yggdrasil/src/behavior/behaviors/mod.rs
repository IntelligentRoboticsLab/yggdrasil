//! All the behaviors that the robot can execute.

mod catchfall;
mod initial;
mod observe;
mod penalized;
mod setup;
mod standup;
mod startup;
mod unstiff;
mod walk;

pub use catchfall::CatchFall;
pub use initial::Initial;
pub use observe::{Observe, ObserveBehaviorConfig};
pub use penalized::Penalized;
pub use setup::Setup;
pub use standup::Standup;
pub use startup::StartUp;
pub use unstiff::Unstiff;
pub use walk::Walk;
