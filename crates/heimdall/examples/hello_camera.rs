use heimdall::{Camera, Result};
use std::{env, path::Path};

fn main() -> Result<()> {
    env::set_var("RUST_BACKTRACE", "1");

    let mut camera = Camera::new_from_path(std::path::Path::new("/dev/video0"))?;

    for _ in 0..10 {
        camera.save_rgb_image_to_file(Path::new("camera_rgb_out.data"))?;
        camera.save_greyscale_image_to_file(Path::new("camera_grey_out.data"))?;
        camera.save_rgb_image_as_jpeg(Path::new("camera_rgb_out.jpg"))?;
    }
    // camera.save_rgb_image_to_file(Path::new("camera_rgb_out.data"))?;
    // camera_handler.save_greyscale_image_to_file(Path::new("camera_grey_out.data"))?;

    // let mut buf_rgb = Vec::with_capacity((NAO_V6_CAMERA_HEIGHT * NAO_V6_CAMERA_WIDTH * 3) as usize);
    // buf_rgb.resize((NAO_V6_CAMERA_WIDTH * NAO_V6_CAMERA_HEIGHT * 3) as usize, 0);
    // let mut output_file = std::fs::File::create(Path::new("camera_rgb_out.data"))?;
    // camera_handler.save_rgb_image(&mut buf_rgb.as_mut_slice())?;
    // output_file.write_all(&mut buf_rgb.as_slice())?;

    // let mut buf_grey = Vec::with_capacity((NAO_V6_CAMERA_HEIGHT * NAO_V6_CAMERA_WIDTH) as usize);
    // buf_grey.resize((NAO_V6_CAMERA_WIDTH * NAO_V6_CAMERA_HEIGHT) as usize, 0);
    // let mut output_file = std::fs::File::create(Path::new("camera_grey_out.data"))?;
    // camera_handler.save_greyscale_image(&mut buf_grey.as_mut_slice())?;
    // output_file.write_all(&mut buf_grey.as_slice())?;

    Ok(())
}
