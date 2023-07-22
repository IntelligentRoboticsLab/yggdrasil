pub mod filter;
pub mod motion;
pub mod nao;

use color_eyre::Result;
use filter::{button::ButtonFilter, force_sensitive_resistor::ForceSensitiveResistorFilter};
use motion::MotionModule;
use nao::NaoModule;
use tyr::prelude::*;

fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    App::new()
        .add_module(NaoModule)?
        .add_module(ButtonFilter)?
        .add_module(ForceSensitiveResistorFilter)?
        .add_module(MotionModule)?
        .run()?;

    Ok(())
}
