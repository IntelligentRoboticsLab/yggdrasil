/// The width of a NAO [`Image`]
pub const IMAGE_WIDTH: u32 = 1280;

/// The height of a NAO [`Image`]
pub const IMAGE_HEIGHT: u32 = 960;

const DEFAULT_CAMERA_CONFIG: Config = Config {
    interval: (1, 30),
    resolution: (IMAGE_WIDTH, IMAGE_HEIGHT),
    format: b"YUYV",
    field: FIELD_NONE,
    nbuffers: 2,
};

use std::{fs::File, io::Write, ops::Deref};

use crate::Result;

use rscam::{Config, Frame, FIELD_NONE};

/// An object that holds a YUYV NAO camera image.
///
/// The image has a width of [`IMAGE_WIDTH`] and a height of [`IMAGE_HEIGHT`].
pub struct Image {
    frame: Frame,
}

fn yuyv_to_rgb(source: &[u8], mut destination: impl Write) -> Result<()> {
    fn clip(value: i32) -> u8 {
        #[allow(clippy::cast_sign_loss)]
        #[allow(clippy::cast_possible_truncation)]
        return i32::min(i32::max(value, 0), 255) as u8;
    }

    fn yuyv422_to_rgb(y1: u8, u: u8, y2: u8, v: u8) -> ((u8, u8, u8), (u8, u8, u8)) {
        let y1 = i32::from(y1) - 16;
        let u = i32::from(u) - 128;
        let y2 = i32::from(y2) - 16;
        let v = i32::from(v) - 128;

        let red1 = (298 * y1 + 409 * v + 128) >> 8;
        let green1 = (298 * y1 - 100 * u - 208 * v + 128) >> 8;
        let blue1 = (298 * y1 + 516 * u + 128) >> 8;

        let red2 = (298 * y2 + 409 * v + 128) >> 8;
        let green2 = (298 * y2 - 100 * u - 208 * v + 128) >> 8;
        let blue2 = (298 * y2 + 516 * u + 128) >> 8;

        (
            (clip(red1), clip(green1), clip(blue1)),
            (clip(red2), clip(green2), clip(blue2)),
        )
    }

    let num_pixels = (IMAGE_WIDTH * IMAGE_HEIGHT) as usize;

    for pixel_duo_id in 0..(num_pixels / 2) {
        let input_offset: usize = (num_pixels / 2 - pixel_duo_id - 1) * 4;
        // Use this if the image should not be flipped.
        // let input_offset: usize = pixel_duo_id * 4;

        let y1 = source[input_offset];
        let u = source[input_offset + 1];
        let y2 = source[input_offset + 2];
        let v = source[input_offset + 3];

        let ((red1, green1, blue1), (red2, green2, blue2)) = yuyv422_to_rgb(y1, u, y2, v);

        destination.write_all(&[red2, green2, blue2, red1, green1, blue1])?;
    }

    Ok(())
}

impl Image {
    /// Return the timestamp of the image in microseconds.
    #[must_use]
    pub fn timestamp(&self) -> u64 {
        self.frame.get_timestamp()
    }

    /// Store the image as a jpeg to a file
    ///
    /// # Errors
    /// This function fails if it cannot convert the taken image, or if it cannot write to the
    /// file.
    pub fn store_jpeg(&self, file_path: &str) -> Result<()> {
        let output_file = File::create(file_path)?;
        let mut encoder = image::codecs::jpeg::JpegEncoder::new(output_file);

        let mut rgb_buffer = Vec::<u8>::with_capacity((IMAGE_WIDTH * IMAGE_HEIGHT * 3) as usize);

        yuyv_to_rgb(&self.frame[..], &mut rgb_buffer)?;

        encoder.encode(
            rgb_buffer.as_slice(),
            IMAGE_WIDTH,
            IMAGE_HEIGHT,
            image::ColorType::Rgb8,
        )?;

        Ok(())
    }

    /// Convert this [`Image`] to RGB and store it in `destination`.
    ///
    /// # Errors
    /// This function fails if it cannot completely write the RGB image to `destination`.
    pub fn to_rgb(&self, destination: impl Write) -> Result<()> {
        yuyv_to_rgb(&self.frame[..], destination)
    }

    /// Extracts a slice containing the entire vector.
    ///
    /// Equivalent to &image[..].
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.frame[..]
    }
}

impl Deref for Image {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.frame
    }
}

/// Struct for retrieving images from the NAO camera.
pub struct Camera {
    camera: rscam::Camera,
}

impl Camera {
    /// Create a new camera object.
    ///
    /// # Errors
    /// This function fails if the [`Camera`] cannot be opened.
    pub fn new(device_path: &str) -> Result<Self> {
        let mut camera = rscam::Camera::new(device_path)?;
        camera.start(&DEFAULT_CAMERA_CONFIG)?;

        Ok(Self { camera })
    }

    /// Get the next image.
    ///
    /// # Errors
    /// This function fails if the [`Camera`] cannot take an image.
    pub fn get_image(&mut self) -> Result<Image> {
        let frame = self.camera.capture()?;

        Ok(Image { frame })
    }
}
