pub mod audio;
pub mod behavior;
pub mod camera;
pub mod filter;
pub mod game_controller;
pub mod leds;
pub mod nao;
pub mod primary_state;

pub use miette::Result;

/// The yggdrasil prelude conveniently includes commonly needed types and traits for writing code in the framework
pub mod prelude {
    pub use crate::Result;
    pub use tyr::prelude::*;
}
