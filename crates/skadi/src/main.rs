pub mod motion_capture;

use crate::motion_capture::Sk;
use yggdrasil::{
    config::ConfigModule,
    filter::FilterModule,
    // leds::LedsModule,
    motion::MotionModule,
    nao::NaoModule,
    prelude::*,
};

fn main() -> Result<()> {
    miette::set_panic_hook();

    let app = App::new()
        .add_module(NaoModule)?
        .add_module(ConfigModule)?
        .add_module(FilterModule)?
        .add_module(MotionModule)?
        // .add_module(LedsModule)?;
        .add_module(SkadiModule)?;

    app.run()
}

// Leds module calls behaviour engine, thus making it crash. This will be a fix for later.
