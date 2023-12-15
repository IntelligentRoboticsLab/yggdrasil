use tyr::{prelude::*, tasks::TaskModule};

use miette::Result;

use yggdrasil::{
    audio::{sound_manager::SoundManagerModule, wee_sound::WeeSoundModule},
    behaviour::BehaviourModule,
    filter::FilterModule,
    motion::MotionModule,
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
        .add_module(MotionModule)?
        .add_module(BehaviourModule)?
        .run()?;
    Ok(())
}
