use miette::Result;
use tyr::tasks::TaskModule;

use yggdrasil::{
<<<<<<< HEAD
    audio::{sound_manager::SoundManagerModule, wee_sound::WeeSoundModule},
    behaviour::BehaviourModule,
    filter::FilterModule,
    motion::MotionModule,
    nao::NaoModule,
=======
    behavior::BehaviorModule, camera::CameraModule, config::ConfigModule, debug::DebugModule,
    filter::FilterModule, game_controller::GameControllerModule, leds::LedsModule,
    motion::MotionModule, nao::NaoModule, prelude::*, primary_state::PrimaryStateModule,
    walk::WalkingEngineModule,
>>>>>>> ceeff45ea380ffba4d81a2169e6c3717906344fd
};

fn main() -> Result<()> {
    tracing_subscriber::fmt::fmt().init();

    miette::set_panic_hook();

    let app = App::new()
        .add_module(NaoModule)?
        .add_module(ConfigModule)?
        .add_module(FilterModule)?
<<<<<<< HEAD
        .add_module(SoundManagerModule)?
        .add_module(WeeSoundModule)?
        .add_module(MotionModule)?
        .add_module(BehaviourModule)?
        .run()?;
    Ok(())
=======
        .add_module(CameraModule)?
        .add_module(MotionModule)?
        .add_module(BehaviorModule)?
        .add_module(LedsModule)?
        .add_module(PrimaryStateModule)?
        .add_module(GameControllerModule)?
        .add_module(WalkingEngineModule)?
        .add_module(DebugModule)?;

    #[cfg(feature = "alsa")]
    let app = app.add_module(yggdrasil::audio::sound_manager::SoundManagerModule)?;

    app.run()
>>>>>>> ceeff45ea380ffba4d81a2169e6c3717906344fd
}
