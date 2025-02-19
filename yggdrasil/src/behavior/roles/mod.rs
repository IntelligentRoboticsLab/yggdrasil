//! All the roles that the robot can take.

mod defender;
mod goalkeeper;
// mod instinct;
mod striker;

pub use defender::{Defender, DefenderRolePlugin};
pub use goalkeeper::{Goalkeeper, GoalkeeperRolePlugin};
// pub use instinct::{Instinct, InstinctRolePlugin};
pub use striker::{Striker, StrikerRolePlugin};
