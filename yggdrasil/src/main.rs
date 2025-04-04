use bevy::state::app::StatesPlugin;
use miette::{Context, IntoDiagnostic};
use tracing::Level;
use tracing_subscriber::fmt::writer::MakeWriterExt;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};
use yggdrasil::prelude::Result;
use yggdrasil::{
    behavior, communication, core, game_controller, kinematics, localization, motion, nao,
    schedule, sensor, vision,
};

use bevy::prelude::*;

fn main() -> Result<()> {
    setup_tracing()?;
    miette::set_panic_hook();

    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .add_plugins((
            schedule::NaoSchedulePlugin,
            game_controller::GameControllerPlugin,
            nao::NaoPlugins,
            tasks::TaskPlugin,
            ml::MlPlugin,
            core::CorePlugins,
            localization::LocalizationPlugin,
            sensor::SensorPlugins,
            behavior::BehaviorPlugins,
            communication::CommunicationPlugins,
            kinematics::KinematicsPlugin,
            motion::MotionPlugins,
            vision::VisionPlugins,
        ));

    bevy_mod_debugdump::print_schedule_graph(&mut app, Update);
    // app.run();
    Ok(())
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
        .with(fmt::Layer::default().with_writer(stdout.and(logfile)))
        .with(symphonia_filter)
        .init();

    Ok(())
}
