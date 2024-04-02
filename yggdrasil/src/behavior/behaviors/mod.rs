//! All the behaviors that the robot can execute.

mod initial;
mod passive;

pub use initial::{Initial, InitialBehaviorConfig};
pub use passive::Passive;
