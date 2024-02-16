#[cfg(feature = "alsa")]
pub mod audio;
<<<<<<< HEAD
pub mod behaviour;
pub mod filter;
=======
pub mod behavior;
pub mod camera;
pub mod config;
pub mod debug;
pub mod filter;
pub mod game_controller;
pub mod kinematics;
pub mod leds;
>>>>>>> ceeff45ea380ffba4d81a2169e6c3717906344fd
pub mod motion;
pub mod nao;
pub mod primary_state;
pub mod walk;

pub use miette::Result;

/// The yggdrasil prelude conveniently includes commonly needed types and traits for writing code in the framework
pub mod prelude {
    pub use crate::{config::ConfigResource, Result};
    pub use odal::Config;
    pub use tyr::prelude::*;
}
