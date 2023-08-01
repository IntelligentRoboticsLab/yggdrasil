//! Contains functions for simplifying the use of a V4L2
//!
//! Written for the Dutch Nao Team as part of project Yggdrasil
//! <https://github.com/intelligentroboticslab>
//!
//! by O. Bosgraaf

use std::{
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use crate::Result;

/// The camera width of a NAO v6.
const NAO_CAMERA_WIDTH: u32 = 1280;

/// The camera height of a NAO v6.
const NAO_CAMERA_HEIGHT: u32 = 960;

use linuxvideo::{
    format::{PixFormat, Pixelformat},
    stream::ReadStream,
    Device,
};

/// Struct for retrieving images from the NAO camera.
pub struct Camera {
    pix_format: PixFormat,
    camera_stream: ReadStream,
}

impl Camera {
    /// Create a new camera object from a path to the camera device.
    pub fn new_from_path(camera_path: &Path) -> Result<Self> {
        let requested_pix_format = linuxvideo::format::PixFormat::new(
            NAO_CAMERA_WIDTH,
            NAO_CAMERA_HEIGHT,
            linuxvideo::format::Pixelformat::YUYV,
        );
        let video_capture = Device::open(camera_path)?.video_capture(requested_pix_format)?;
        let pix_format = video_capture.format();

        Ok(Self {
            pix_format: PixFormat::new(
                pix_format.width(),
                pix_format.height(),
                pix_format.pixelformat(),
            ),
            camera_stream: video_capture.into_stream(1)?,
        })
    }

    /// Create a new camera object from a camera device.
    pub fn new_from_device(camera: Device, requested_pix_format: PixFormat) -> Result<Self> {
        let video_capture = camera.video_capture(requested_pix_format)?;
        let pix_format = video_capture.format();

        Ok(Self {
            pix_format: PixFormat::new(
                pix_format.width(),
                pix_format.height(),
                pix_format.pixelformat(),
            ),
            camera_stream: video_capture.into_stream(4)?,
        })
    }

    /// Get the width of the images taken by this [Camera] object.
    pub fn image_width(&self) -> u32 {
        self.pix_format.width()
    }

    /// Get the height of the images taken by this [Camera] object.
    pub fn image_height(&self) -> u32 {
        self.pix_format.height()
    }

    fn yuyv444_to_rgb(y: u8, u: u8, v: u8) -> (u8, u8, u8) {
        fn clip(value: i32) -> u8 {
            i32::max(0, i32::min(255, value)) as u8
        }

        let c = y as i32 - 16;
        let d = u as i32 - 128;
        let e = v as i32 - 128;

        let red = (298 * c + 409 * e + 128) >> 8;
        let green = (298 * c - 100 * d - 208 * e + 128) >> 8;
        let blue = (298 * c + 516 * d + 128) >> 8;

        (clip(red), clip(green), clip(blue))
    }

    fn save_rgb_image_from_yuyv(&mut self, mut destination: impl Write) -> Result<()> {
        let num_pixels = (self.pix_format.width() * self.pix_format.height()) as usize;

        let stream = &mut self.camera_stream;
        stream.dequeue(|image_buffer_yuv_422| {
            for pixel_duo_id in 0..(num_pixels / 2) {
                let input_offset: usize = (num_pixels / 2 - pixel_duo_id - 1) * 4;
                // let input_offset: usize = pixel_duo_id * 4;

                let y1 = image_buffer_yuv_422[input_offset];
                let u = image_buffer_yuv_422[input_offset + 1];
                let y2 = image_buffer_yuv_422[input_offset + 2];
                let v = image_buffer_yuv_422[input_offset + 3];

                let (red1, green1, blue1) = Self::yuyv444_to_rgb(y1, u, v);
                let (red2, green2, blue2) = Self::yuyv444_to_rgb(y2, u, v);

                destination.write_all(&[red2, green2, blue2, red1, green1, blue1])?;
            }

            Ok(())
        })?;

        Ok(())
    }

    /// Save a raw RGB photo to the buffer.
    ///
    /// The buffer `destination` should have a size of at least
    /// [`image_width`](Camera::image_width) * [`image_height`](Camera::image_height) * 3 bytes.
    pub fn save_rgb_image(&mut self, destination: &mut [u8]) -> Result<()> {
        match self.pix_format.pixelformat() {
            Pixelformat::YUYV => self.save_rgb_image_from_yuyv(destination),
            pixel_format => Ok(eprintln!("Unsupported pixel format: {pixel_format}")),
        }
    }

    /// Save an RGB image to a file.
    ///
    /// The resuling file is a raw stream of bytes, each three bytes representing a single pixel.
    pub fn save_rgb_image_to_file(&mut self, destination: &Path) -> Result<()> {
        let output_file = File::create(destination)?;
        let mut output_file_buffer = BufWriter::with_capacity(4096, output_file);

        match self.pix_format.pixelformat() {
            Pixelformat::YUYV => self.save_rgb_image_from_yuyv(&mut output_file_buffer),
            pixel_format => Ok(eprintln!("Unsupported pixel format: {pixel_format}")),
        }
    }

    fn save_greyscale_image_from_yuyv(&mut self, mut destination: impl Write) -> Result<()> {
        let num_pixels = (self.pix_format.width() * self.pix_format.height()) as usize;

        self.camera_stream.dequeue(|image_buffer_yuv_422| {
            for pixel_duo_id in 0..(num_pixels / 2) {
                let input_offset: usize = (num_pixels / 2 - pixel_duo_id - 1) * 4;
                // let input_offset: usize = pixel_duo_id * 4;

                let y1 = image_buffer_yuv_422[input_offset];
                let y2 = image_buffer_yuv_422[input_offset + 2];

                destination.write_all(&[y1, y2])?;
            }

            Ok(())
        })?;

        Ok(())
    }

    /// Save a greyscale photo to the buffer.
    ///
    /// The buffer `destination` should have a size of at least
    /// [`image_width`](Camera::image_width) * [`image_height`](Camera::image_height) * 3 bytes.
    pub fn save_greyscale_image(&mut self, destination: &mut [u8]) -> Result<()> {
        match self.pix_format.pixelformat() {
            Pixelformat::YUYV => self.save_greyscale_image_from_yuyv(destination),
            pixel_format => Ok(eprintln!("Unsupported pixel format: {pixel_format}")),
        }
    }

    /// Save a greyscale image to a file.
    ///
    /// The resuling file is a raw stream of bytes, each byte representing a single pixel.
    pub fn save_greyscale_image_to_file(&mut self, destination: &Path) -> Result<()> {
        let output_file = File::create(destination)?;
        let output_file_buffer = BufWriter::with_capacity(4096, output_file);

        match self.pix_format.pixelformat() {
            Pixelformat::YUYV => self.save_greyscale_image_from_yuyv(output_file_buffer),
            pixel_format => Ok(eprintln!("Unsupported pixel format: {pixel_format}")),
        }
    }
}
