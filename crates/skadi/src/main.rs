pub mod motion_capture;

use yggdrasil::{
    config::ConfigModule, filter::FilterModule, leds::LedsModule, motion::MotionModule, nao::NaoModule, prelude::*
};
use crate::motion_capture::Sk;

fn main() -> Result<()> {
    miette::set_panic_hook();

    let app = App::new()
        .add_module(NaoModule)?
        .add_module(ConfigModule)?
        .add_module(FilterModule)?
        .add_module(MotionModule)?
        .add_module(LedsModule)?
        .add_module(Sk)?;

    app.run()
}
