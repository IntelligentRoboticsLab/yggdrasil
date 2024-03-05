use miette::Result;
// use tyr::tasks::TaskModule;

use yggdrasil::{
    behavior::BehaviorModule, camera::CameraModule, config::ConfigModule, debug::DebugModule,
    filter::FilterModule, game_controller::GameControllerModule, leds::LedsModule,
    motion::MotionModule, nao::NaoModule, prelude::*, primary_state::PrimaryStateModule,
    walk::WalkingEngineModule,
};

fn main() -> Result<()> {
    tracing_subscriber::fmt::fmt().init();

    miette::set_panic_hook();

    let app = App::new()
        .add_module(NaoModule)?
        .add_module(ConfigModule)?
        .add_module(FilterModule)?
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
}
