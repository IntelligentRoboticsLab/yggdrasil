use std::io::Write;

use heimdall::{Camera, Result, RgbImage, CAMERA_TOP};

fn main() -> Result<()> {
    let mut camera = Camera::new(CAMERA_TOP)?;
    let image = camera.get_yuyv_image()?;

    let mut file = std::fs::File::create("yuyv_image.raw")?;
    file.write_all(&image)?;

    let mut rgb_image = RgbImage::new();
    image.to_rgb(&mut rgb_image)?;

    image.store_jpeg("image.jpeg")?;

    Ok(())
}
