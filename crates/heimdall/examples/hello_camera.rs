use heimdall::{camera::*, Result, NAO_V6_CAMERA_HEIGHT, NAO_V6_CAMERA_WIDTH};
use std::{env, path::Path};

fn main() -> Result<()> {
    env::set_var("RUST_BACKTRACE", "1");

    let mut camera = Camera::new_from_path(
        std::path::Path::new("/dev/video-top"),
        // std::path::Path::new("/dev/video-bottom"),
        linuxvideo::format::PixFormat::new(
            NAO_V6_CAMERA_WIDTH,
            NAO_V6_CAMERA_HEIGHT,
            linuxvideo::format::Pixelformat::YUYV,
        ),
    )?;

    for _ in 0..10 {
        camera.save_rgb_screenshot_to_file(Path::new("camera_rgb_out.data"))?;
    }
    // camera.save_rgb_screenshot_to_file(Path::new("camera_rgb_out.data"))?;
    // camera_handler.save_greyscale_screenshot_to_file(Path::new("camera_grey_out.data"))?;

    // let mut buf_rgb = Vec::with_capacity((NAO_V6_CAMERA_HEIGHT * NAO_V6_CAMERA_WIDTH * 3) as usize);
    // buf_rgb.resize((NAO_V6_CAMERA_WIDTH * NAO_V6_CAMERA_HEIGHT * 3) as usize, 0);
    // let mut output_file = std::fs::File::create(Path::new("camera_rgb_out.data"))?;
    // camera_handler.save_rgb_screenshot(&mut buf_rgb.as_mut_slice())?;
    // output_file.write_all(&mut buf_rgb.as_slice())?;

    // let mut buf_grey = Vec::with_capacity((NAO_V6_CAMERA_HEIGHT * NAO_V6_CAMERA_WIDTH) as usize);
    // buf_grey.resize((NAO_V6_CAMERA_WIDTH * NAO_V6_CAMERA_HEIGHT) as usize, 0);
    // let mut output_file = std::fs::File::create(Path::new("camera_grey_out.data"))?;
    // camera_handler.save_greyscale_screenshot(&mut buf_grey.as_mut_slice())?;
    // output_file.write_all(&mut buf_grey.as_slice())?;

    Ok(())
}
