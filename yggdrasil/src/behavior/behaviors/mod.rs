//! All the behaviors that the robot can execute.

mod catchfall;
mod observe;
mod stand;
mod stand_look;
mod standup;
mod startup;
mod unstiff;
mod walk;
mod walk_to_set;

pub use catchfall::CatchFall;
pub use observe::{Observe, ObserveBehaviorConfig};
pub use stand::Stand;
pub use stand_look::StandLookAt;
pub use standup::Standup;
pub use startup::StartUp;
pub use unstiff::Unstiff;
pub use walk::Walk;
pub use walk_to_set::WalkToSet;
