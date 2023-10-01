use std::io::Write;

use heimdall::{Camera, Result, RgbImage, IMAGE_HEIGHT, IMAGE_WIDTH};

fn main() -> Result<()> {
    let mut camera = Camera::new("/dev/video0")?;
    let image = camera.get_yuyv_image()?;

    let mut file = std::fs::File::create("image.raw")?;
    file.write_all(&image[..])?;

    let mut rgb_image = RgbImage::new(IMAGE_WIDTH, IMAGE_HEIGHT);
    for _ in 0..100 {
        image.to_rgb(&mut rgb_image)?;
    }

    image.store_jpeg("image.jpeg")?;

    Ok(())
}
