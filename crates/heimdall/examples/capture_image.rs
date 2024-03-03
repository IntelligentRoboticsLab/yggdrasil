use std::io::Write;

use heimdall::{Camera, CameraDevice, Result};

fn main() -> Result<()> {
    let camera_device = CameraDevice::new("/dev/video-top")?;
    camera_device.horizontal_flip()?;
    camera_device.vertical_flip()?;

    let mut camera = Camera::new(camera_device, 640, 480, 3)?;

    let image = camera.get_yuyv_image()?;

    let mut file = std::fs::File::create("yuyv_image.raw")?;
    file.write_all(&image)?;

    let _rgb_image = image.to_rgb()?;

    image.store_jpeg("image.jpeg")?;

    Ok(())
}
