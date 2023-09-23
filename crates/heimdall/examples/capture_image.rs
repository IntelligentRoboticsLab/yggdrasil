use std::io::Write;

use heimdall::{Camera, Result};

fn main() -> Result<()> {
    let mut camera = Camera::new("/dev/video0")?;
    let image = camera.get_image()?;

    let mut file = std::fs::File::create("image.raw")?;
    file.write_all(&image[..])?;

    image.store_jpeg("image.jpeg")?;

    Ok(())
}
