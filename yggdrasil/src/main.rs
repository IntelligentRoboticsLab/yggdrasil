pub mod audio;
pub mod filter;
pub mod nao;

use audio::{sound_manager::SoundManagerModule, wee_sound::WeeSoundModule};
use filter::FilterModule;
use miette::Result;
use nao::NaoModule;
use tyr::{prelude::*, tasks::TaskModule};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    miette::set_panic_hook();

    App::new()
        .add_module(TaskModule)?
        .add_module(NaoModule)?
        .add_module(FilterModule)?
        .add_module(SoundManagerModule)?
        .add_module(WeeSoundModule)?
        .run()?;
    Ok(())
}
