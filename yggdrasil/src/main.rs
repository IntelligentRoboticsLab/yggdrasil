use yggdrasil::{
<<<<<<< HEAD
    behavior::BehaviorModule, camera::CameraModule, config::ConfigModule, filter::FilterModule,
    game_controller::GameControllerModule, leds::LedsModule, nao::NaoModule, prelude::*,
    primary_state::PrimaryStateModule, walk::WalkingEngineModule,
=======
    audio::{sound_manager::SoundManagerModule, wee_sound::WeeSoundModule},
    behavior::BehaviorModule,
    camera::CameraModule,
    debug::DebugModule,
    filter::FilterModule,
    game_controller::GameControllerModule,
    leds::LedsModule,
    nao::NaoModule,
    prelude::*,
    primary_state::PrimaryStateModule,
>>>>>>> e7e01e3 (basic start of implementing debug panel)
};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    miette::set_panic_hook();

    let app = App::new()
        .add_module(NaoModule)?
        .add_module(ConfigModule)?
        .add_module(FilterModule)?
        .add_module(CameraModule)?
        .add_module(BehaviorModule)?
        .add_module(LedsModule)?
        .add_module(PrimaryStateModule)?
        .add_module(GameControllerModule)?
        .add_module(WalkingEngineModule)?
        .add_module(DebugModule)?;

    #[cfg(feature = "alsa")]
    let app = app.add_module(yggdrasil::audio::sound_manager::SoundManagerModule)?;

    app.run()
}
