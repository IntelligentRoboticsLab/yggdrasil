pub mod motion_capture;

use crate::motion_capture::SkadiModule;
use yggdrasil::{core::config::ConfigModule, sensor::imu::IMUSensor, nao::NaoModule, prelude::*};

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    miette::set_panic_hook();

    let mut app = App::new().add_module(NaoModule)?;
    app = app.add_module(ConfigModule)?;
    app = app.add_module(IMUSensor)?;
    // .add_module(MotionModule)?
    app = app.add_module(SkadiModule)?;

    app.run()
}

// Leds module calls behaviour engine, thus making it crash. This will be a fix for later. UPDATE: MotionModule calls walking engine, thus making it crash. This will be a fix for later