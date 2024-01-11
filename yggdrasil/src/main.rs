use yggdrasil::{
    behavior::BehaviorModule, camera::CameraModule, config::ConfigModule, filter::FilterModule,
    game_controller::GameControllerModule, leds::LedsModule, nao::NaoModule, prelude::*,
    primary_state::PrimaryStateModule,
    vision::VisionModule,
};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    miette::set_panic_hook();

<<<<<<< HEAD
    let app = App::new()
        .add_module(NaoModule)?
        .add_module(ConfigModule)?
        .add_module(FilterModule)?
        .add_module(CameraModule)?
        .add_module(BehaviorModule)?
        .add_module(LedsModule)?
        .add_module(PrimaryStateModule)?
        .add_module(GameControllerModule)?
        .add_module(VisionModule)?;


    #[cfg(feature = "alsa")]
    let app = app.add_module(yggdrasil::audio::sound_manager::SoundManagerModule)?;

    app.run()
=======
    App::new()
        .add_module(TaskModule)?
        // .add_module(NaoModule)?
        // .add_module(FilterModule)?
        // .add_module(SoundManagerModule)?
        // .add_module(WeeSoundModule)?
        .add_module(CameraModule)?
        .add_module(VisionModule)?
        // .add_module(BehaviorModule)?
        // .add_module(LedsModule)?
        // .add_module(PrimaryStateModule)?
        .run()
>>>>>>> d5e3abf (Update camera paths and optimize line detection with changes from my branch)
}
