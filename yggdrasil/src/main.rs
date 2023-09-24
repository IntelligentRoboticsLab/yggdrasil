use tyr::{prelude::*, tasks::TaskModule};

use miette::Result;

use heimdall::Camera;
use yggdrasil::{
    audio::{sound_manager::SoundManagerModule, wee_sound::WeeSoundModule},
    filter::FilterModule,
    nao::NaoModule,
};

fn main() -> Result<()> {
    tracing_subscriber::fmt::fmt().pretty().init();

    miette::set_panic_hook();

    let mut camera = Camera::new("/dev/video0").unwrap();

    for i in 0..10 {
        let image = camera.get_image().unwrap();
        image.store_jpeg(&format!("image-{}.jpeg", i)).unwrap();
    }

    App::new()
        .add_module(TaskModule)?
        .add_module(NaoModule)?
        .add_module(FilterModule)?
        .add_module(SoundManagerModule)?
        .add_module(WeeSoundModule)?
        .run()?;
    Ok(())
}
