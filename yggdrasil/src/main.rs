use yggdrasil::localization::LocalizationModule;
#[allow(unused_imports)]
use yggdrasil::{
    behavior::BehaviorModule, core::config::ConfigModule, core::debug::DebugModule,
    core::ml::MlModule, game_controller::GameControllerModule, kinematics::KinematicsModule,
    motion::walk::WalkingEngineModule, motion::MotionModule, nao::NaoModule, prelude::*,
    sensor::FilterModule, vision::camera::CameraModule, vision::VisionModule,
};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    miette::set_panic_hook();

    let app = App::new()
        .add_module(NaoModule)?
        .add_module(ConfigModule)?
        .add_module(MlModule)?
        .add_module(FilterModule)?
        .add_module(KinematicsModule)?
        .add_module(CameraModule)?
        .add_module(BehaviorModule)?
        .add_module(GameControllerModule)?
        .add_module(WalkingEngineModule)?
        .add_module(DebugModule)?
        .add_module(VisionModule)?
        .add_module(MotionModule)?
        .add_module(LocalizationModule)?;

    #[cfg(feature = "alsa")]
    let app = app.add_module(yggdrasil::core::core::audio::AudioModule)?;

    app.run()
}
