pub mod audio;
pub mod filter;
pub mod nao;

use color_eyre::Result;
use filter::FilterModule;
use nao::NaoModule;
use tyr::prelude::*;

fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    App::new()
        .add_module(NaoModule)?
        .add_module(FilterModule)?
        .add_module(ButtonFilter)?
        .add_module(ForceSensitiveResistorFilter)?
        .add_module(IMUFilter)?
        .add_module(SonarFilter)?
        .add_module(SoundManagerModule)?
        .run()?;
    Ok(())
}
