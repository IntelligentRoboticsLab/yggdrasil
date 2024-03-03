#[allow(unused_imports)]
use yggdrasil::{
    behavior::BehaviorModule, camera::CameraModule, config::ConfigModule, debug::DebugModule,
    filter::FilterModule, game_controller::GameControllerModule, leds::LedsModule, nao::NaoModule,
    mltask::MLModule, prelude::*, primary_state::PrimaryStateModule,
    walk::WalkingEngineModule, vision::VisionModule
};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    miette::set_panic_hook();

    let app = App::new()
        .add_module(NaoModule)?
        .add_module(MLModule)?
        .add_module(ConfigModule)?
        .add_module(FilterModule)?
        .add_module(CameraModule)?
        .add_module(BehaviorModule)?
        .add_module(LedsModule)?
        .add_module(PrimaryStateModule)?
        .add_module(GameControllerModule)?
        .add_module(WalkingEngineModule)?
        .add_module(DebugModule)?
        .add_module(VisionModule)?;

    #[cfg(feature = "alsa")]
    let app = app.add_module(yggdrasil::audio::AudioModule)?;

    app.run()
}
