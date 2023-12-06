#[cfg(feature = "lola")]
use tyr::{prelude::*, tasks::TaskModule};

use miette::Result;
use yggdrasil::{
    audio::sound_manager::SoundManagerModule, audio::wee_sound::WeeSoundModule,
    debug::WebSocketModule, filter::FilterModule, nao::NaoModule,
};

#[cfg(feature = "lola")]
fn main() -> Result<()> {
    tracing_subscriber::fmt::fmt().init();

    miette::set_panic_hook();

    App::new()
        .add_module(TaskModule)?
        // .add_module(NaoModule)?
        // .add_module(FilterModule)?
        // .add_module(SoundManagerModule)?
        // .add_module(WeeSoundModule)?
        .add_module(WebSocketModule)?
        .run()?;
    Ok(())
}
