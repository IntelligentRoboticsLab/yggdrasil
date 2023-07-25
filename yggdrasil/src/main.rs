pub mod audio;
pub mod filter;
pub mod nao;

use audio::sound_manager::SoundManagerModule;

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
        .add_module(SoundManagerModule)?
        .run()?;
    Ok(())
}
