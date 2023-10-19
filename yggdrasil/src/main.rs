pub mod audio;
pub mod filter;
pub mod nao;

use audio::{sound_manager::SoundManagerModule, wee_sound::WeeSoundModule};
use tyr::{prelude::*, tasks::TaskModule};

use miette::Result;
use yggdrasil::{debug::WebSocketModule, filter::FilterModule, nao::NaoModule};

fn main() -> Result<()> {
    tracing_subscriber::fmt::fmt().init();

    miette::set_panic_hook();

    App::new()
        .add_module(TaskModule)?
        .add_module(NaoModule)?
        .add_module(FilterModule)?
        .add_module(SoundManagerModule)?
        .add_module(WeeSoundModule)?
        .add_module(WebSocketModule)?
        .run()?;
    Ok(())
}
