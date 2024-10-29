//! All the roles that the robot can take.

mod attacker;
mod defender;
mod keeper;

pub use attacker::Striker;
pub use defender::Defender;
pub use keeper::Goalkeeper;
