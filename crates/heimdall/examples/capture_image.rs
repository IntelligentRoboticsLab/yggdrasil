use std::io::Write;

use heimdall::{Camera, Result, CAMERA_TOP};

fn main() -> Result<()> {
    let mut camera = Camera::new(CAMERA_TOP)?;
    let image = camera.get_yuyv_image()?;

    let mut file = std::fs::File::create("yuyv_image.raw")?;
    file.write_all(&image)?;

    let _rgb_image = image.to_rgb()?;

    image.store_jpeg("image.jpeg")?;

    Ok(())
}
