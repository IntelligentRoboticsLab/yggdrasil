use miette::{Context, IntoDiagnostic};
use tracing::Level;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter, Layer};
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
    setup_tracing()?;
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
    let app = app.add_module(yggdrasil::core::audio::AudioModule)?;

    #[cfg(feature = "dependency_graph")]
    return app.store_system_dependency_graph("../dependency_graph.png");

    #[cfg(not(feature = "dependency_graph"))]
    return app.run();
}

fn setup_tracing() -> Result<()> {
    let logfile = tracing_appender::rolling::hourly(
        format!(
            "{}/.local/state/yggdrasil",
            std::env::var("HOME").into_diagnostic()?
        ),
        "yggdrasil.log",
    );
    let stdout = std::io::stdout.with_max_level(tracing::Level::INFO);

    let subscriber = tracing_subscriber::registry();

    #[cfg(feature = "timings")]
    let subscriber = subscriber.with(tracing_tracy::TracyLayer::default());

    // filter out the symphonia probe spam when playing audio
    let symphonia_filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .from_env_lossy()
        .add_directive(
            "symphonia_core::probe=off"
                .parse()
                .into_diagnostic()
                .wrap_err("Failed to parse symphonia probe filter")?,
        );

    subscriber
        .with(
            fmt::Layer::default()
                .with_writer(stdout.and(logfile))
                .with_filter(symphonia_filter),
        )
        .try_init()
        .into_diagnostic()
        .wrap_err("Failed to initialize tracing subscriber")?;

    Ok(())
}
