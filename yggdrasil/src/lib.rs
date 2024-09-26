pub mod behavior;
// pub mod communication;
pub mod core;
// pub mod game_controller;
pub mod game_controller2;
pub mod kinematics;
pub mod localization;
pub mod motion;
pub mod nao;
pub mod schedule;
pub mod sensor;
pub mod vision;

pub use miette::Result;

/// The yggdrasil prelude conveniently includes commonly needed types and traits for writing code
/// in the framework.
pub mod prelude {
    pub use crate::schedule::{PostWrite, PreWrite, Sensor, Write};
    pub use crate::{core::config::ConfigExt, Result};
    pub use odal::Config;
    // pub use tyr::prelude::*;
}
