use yggdrasil::{
    audio::{sound_manager::SoundManagerModule, wee_sound::WeeSoundModule},
    behavior::BehaviorModule,
    camera::CameraModule,
    config::ConfigModule,
    filter::FilterModule,
    game_controller::GameControllerModule,
    leds::LedsModule,
    nao::{NaoModule, RobotInfo},
    prelude::*,
    primary_state::PrimaryStateModule,
};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    miette::set_panic_hook();

    App::new()
        // .add_module(NaoModule)?
        .add_resource(Resource::new(RobotInfo {
            robot_name: "daphne".to_string(),
            robot_id: 26,
            head_id: "X".to_string(),
            head_version: "V6".to_string(),
            body_id: "X".to_string(),
            body_version: "V6".to_string(),
        }))?
        .add_module(ConfigModule)?
        .add_module(TaskModule)?
        // .add_module(FilterModule)?
        // .add_module(SoundManagerModule)?
        // .add_module(WeeSoundModule)?
        // .add_module(CameraModule)?
        // .add_module(BehaviorModule)?
        // .add_module(LedsModule)?
        // .add_module(PrimaryStateModule)?
        // .add_module(GameControllerModule)?
        .run()?;
    Ok(())
}
