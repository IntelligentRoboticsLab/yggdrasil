use tyr::{prelude::*, tasks::TaskModule};

use miette::Result;

use yggdrasil::{
    audio::{sound_manager::SoundManagerModule, wee_sound::WeeSoundModule},
    filter::FilterModule,
    nao::NaoModule,
};

fn main() -> Result<()> {
    tracing_subscriber::fmt::fmt().init();

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
