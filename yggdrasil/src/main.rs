pub mod filter;
pub mod nao;

use color_eyre::Result;
use filter::{
    button::ButtonFilter, force_sensitive_resistor::ForceSensitiveResistorFilter, imu::IMUFilter,
    sonar::SonarFilter,
};
use nao::NaoModule;
use tyr::prelude::*;

fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    App::new()
        .add_module(NaoModule)?
        .add_module(ButtonFilter)?
        .add_module(ForceSensitiveResistorFilter)?
        .add_module(IMUFilter)?
        .add_module(SonarFilter)?
        .run()?;

    Ok(())
}
