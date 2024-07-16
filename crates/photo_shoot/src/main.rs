extern crate yggdrasil;

#[allow(unused_imports)]
use yggdrasil::behavior::BehaviorModule;
use yggdrasil::core::config::ConfigModule;
use yggdrasil::motion::walk::WalkingEngineModule;
use yggdrasil::nao::NaoModule;
use yggdrasil::photo::PhotoModule;
use yggdrasil::prelude::*;
use yggdrasil::sensor::SensorModule;
use yggdrasil::vision::camera::CameraModule;

fn main() -> Result<()> {
    println!("test123");

    tracing_subscriber::fmt::init();

    miette::set_panic_hook();

    let app = App::new()
        .add_module(NaoModule)?
        .add_module(ConfigModule)?
        .add_module(CameraModule)?
        .add_module(SensorModule)?
        .add_module(WalkingEngineModule)?
        .add_module(PhotoModule)?;

    #[cfg(feature = "alsa")]
    let app = app.add_module(yggdrasil::core::audio::AudioModule)?;

    app.run()
}
