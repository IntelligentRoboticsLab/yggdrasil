#[allow(unused_imports)]
use yggdrasil::behavior::BehaviorModule;
use yggdrasil::core::{config::ConfigModule, debug::DebugModule, ml::MlModule};
use yggdrasil::game_controller::GameControllerModule;
use yggdrasil::kinematics::KinematicsModule;
use yggdrasil::localization::LocalizationModule;
use yggdrasil::motion::walk::WalkingEngineModule;
use yggdrasil::motion::MotionModule;
use yggdrasil::nao::NaoModule;
use yggdrasil::prelude::*;
use yggdrasil::sensor::SensorModule;
use yggdrasil::vision::camera::CameraModule;
use yggdrasil::vision::VisionModule;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    miette::set_panic_hook();

    let app = App::new()
        .add_module(NaoModule)?
        .add_module(ConfigModule)?
        .add_module(MlModule)?
        .add_module(SensorModule)?
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
    let app = app.add_module(yggdrasil::core::audio::AudioModule)?;

    // app.store_system_dependency_graph("/tmp/dependency_graph.png")?;

    app.run()
}
