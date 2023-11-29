use tyr::{prelude::*, tasks::TaskModule};

use miette::Result;

use yggdrasil::{
    audio::{sound_manager::SoundManagerModule, wee_sound::WeeSoundModule},
    filter::FilterModule,
    nao::NaoModule,
};

use heimdall::*;

fn main() -> Result<()> {
    tracing_subscriber::fmt::fmt().init();

    miette::set_panic_hook();

    let mut camera = heimdall::Camera::new("/dev/video0").unwrap();
    for _ in 0..10 {
        let yuyv = camera.get_yuyv_image().unwrap();
        yuyv.store_jpeg("out.jpeg").unwrap();
    }

    // App::new()
    //     .add_module(TaskModule)?
    //     .add_module(NaoModule)?
    //     .add_module(FilterModule)?
    //     .add_module(SoundManagerModule)?
    //     .add_module(WeeSoundModule)?
    //     .run()?;
    Ok(())
}
