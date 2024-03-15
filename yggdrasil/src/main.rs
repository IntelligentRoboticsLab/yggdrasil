use tyr::tasks::{TaskConfig, TaskModule};
#[allow(unused_imports)]
use yggdrasil::{
    behavior::BehaviorModule, camera::CameraModule, config::ConfigModule, debug::DebugModule,
    filter::FilterModule, game_controller::GameControllerModule, leds::LedsModule, ml::MlModule,
    nao::NaoModule, prelude::*, primary_state::PrimaryStateModule, vision::VisionModule,
    walk::WalkingEngineModule,
};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    miette::set_panic_hook();

    let task_config = TaskConfig {
        async_threads: 1,
        compute_threads: 1,
    };

    let app = App::new()
        .add_module(NaoModule)?
        .add_module(ConfigModule)?
        .add_resource(Resource::new(task_config))?
        .add_module(TaskModule)?
        .add_module(MlModule)?
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
