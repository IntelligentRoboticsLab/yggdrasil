//! All the behaviors that the robot can execute.

// mod align_with;
mod observe;
mod stand;
mod stand_look;
mod startup;
mod test;
mod unstiff;
mod walk;
mod walk_to;

// pub use align_with::AlignWith;
pub use observe::{Observe, ObserveBehaviorConfig};
pub use stand::Stand;
pub use stand_look::StandingLookAt;
pub use startup::StartUp;
pub use test::Test;
pub use unstiff::Unstiff;
pub use walk::Walk;
pub use walk_to::WalkTo;
