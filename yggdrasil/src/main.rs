use tyr::{prelude::*, tasks::TaskModule};

use miette::Result;

use yggdrasil::{
    audio::{sound_manager::SoundManagerModule, wee_sound::WeeSoundModule},
    behaviour::BehaviourModule,
    filter::FilterModule,
    game_phase::GamePhaseModule,
    leds::LedsModule,
    nao::NaoModule,
};

fn main() -> Result<()> {
    tracing_subscriber::fmt::fmt().init();

    miette::set_panic_hook();

    App::new()
        .add_module(BehaviourModule)?
        .add_module(FilterModule)?
        .add_module(GamePhaseModule)?
        .add_module(LedsModule)?
        .add_module(SoundManagerModule)?
        .add_module(TaskModule)?
        .add_module(NaoModule)?
        .add_module(WeeSoundModule)?
        .run()?;
    Ok(())
}
