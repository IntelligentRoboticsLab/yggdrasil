//! All the roles that the robot can take.

mod defender;
mod goalkeeper;
mod striker;
mod kick_in;

pub use defender::{Defender, DefenderRolePlugin};
pub use goalkeeper::{Goalkeeper, GoalkeeperRolePlugin};
pub use striker::{Striker, StrikerRolePlugin};
pub use kick_in::{KickIn, KickInRolePlugin};
