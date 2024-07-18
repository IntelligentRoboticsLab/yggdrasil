use miette::IntoDiagnostic;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use yggdrasil::behavior::BehaviorModule;
use yggdrasil::communication::CommunicationModule;
use yggdrasil::core::whistle::WhistleStateModule;
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
    let logfile = tracing_appender::rolling::hourly(
        format!(
            "{}/.local/state/yggdrasil",
            std::env::var("HOME").into_diagnostic()?
        ),
        "yggdrasil.log",
    );
    let stdout = std::io::stdout.with_max_level(tracing::Level::INFO);

    tracing_subscriber::fmt()
        .with_writer(stdout.and(logfile))
        .init();

    miette::set_panic_hook();

    let app = App::new()
        .add_module(NaoModule)?
        .add_module(ConfigModule)?
        .add_module(MlModule)?
        .add_module(SensorModule)?
        .add_module(KinematicsModule)?
        .add_module(CameraModule)?
        .add_module(BehaviorModule)?
        .add_module(CommunicationModule)?
        .add_module(GameControllerModule)?
        .add_module(WalkingEngineModule)?
        .add_module(DebugModule)?
        .add_module(VisionModule)?
        .add_module(MotionModule)?
        .add_module(LocalizationModule)?
        .add_module(WhistleStateModule)?;

    #[cfg(feature = "alsa")]
    app.add_module(yggdrasil::core::audio::AudioModule)?;

    #[cfg(feature = "dependency_graph")]
    return app.store_system_dependency_graph("../dependency_graph.png");

    #[cfg(not(feature = "dependency_graph"))]
    return app.run();
}
