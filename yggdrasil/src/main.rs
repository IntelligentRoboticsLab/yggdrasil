use yggdrasil::{
    audio::{sound_manager::SoundManagerModule, wee_sound::WeeSoundModule},
    behavior::BehaviorModule,
    camera::CameraModule,
    config::{yggdrasil::TyrModule, ConfigModule, ConfigResource},
    filter::FilterModule,
    game_controller::GameControllerModule,
    leds::LedsModule,
    nao::NaoModule,
    primary_state::PrimaryStateModule,
};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    miette::set_panic_hook();

    App::new()
        .add_module(NaoModule)?
        .add_module(ConfigModule)?
        .init_module::<TyrModule>()?
        .add_module(FilterModule)?
        .add_module(SoundManagerModule)?
        .add_module(WeeSoundModule)?
        .add_module(CameraModule)?
        .add_module(BehaviorModule)?
        .add_module(LedsModule)?
        .add_module(PrimaryStateModule)?
        .add_module(GameControllerModule)?
        .run()?;
    Ok(())
}
