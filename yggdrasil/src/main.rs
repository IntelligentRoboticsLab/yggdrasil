pub mod audio;
pub mod filter;
pub mod nao;

use audio::{sound_manager::SoundManagerModule, wee_sound::WeeSoundModule};
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
        .add_module(WeeSoundModule)?
        .run()?;
    Ok(())
}
