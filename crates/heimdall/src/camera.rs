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

use simdeez::sse2::*;
use simdeez::sse41::*;

simd_compiletime_generate!(
    pub fn yuyv422_to_rgb2(
        pixels: &[u8],
        destination: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        unsafe fn clip<SS: Simd>(value: SS::Vi32) -> u8 {
            let mut result = 0;

            SS::storeu_epi32(
                &mut result,
                SS::max_epi32(SS::set1_epi32(0), SS::min_epi32(SS::set1_epi32(255), value)),
            );

            result as u8
        }

        let mut pixels: &[u8] = pixels;

        while pixels.len() >= S::VI32_WIDTH {
            let four_bytes = &*(pixels.as_ptr() as *const i32);

            let y1 = ((S::loadu_epi32(four_bytes) >> 0) & S::set1_epi32(255)) - S::set1_epi32(16);
            let u = ((S::loadu_epi32(four_bytes) >> 8) & S::set1_epi32(255)) - S::set1_epi32(128);
            let y2 = ((S::loadu_epi32(four_bytes) >> 16) & S::set1_epi32(255)) - S::set1_epi32(16);
            let v = ((S::loadu_epi32(four_bytes) >> 24) & S::set1_epi32(255)) - S::set1_epi32(128);

            let red1 = (S::set1_epi32(298) * y1 + S::set1_epi32(409) * v + S::set1_epi32(128)) >> 8;
            let green1 =
                (S::set1_epi32(298) * y1 - S::set1_epi32(100) * u - S::set1_epi32(208) * v
                    + S::set1_epi32(128))
                    >> 8;
            let blue1 =
                (S::set1_epi32(298) * y1 + S::set1_epi32(516) * u + S::set1_epi32(128)) >> 8;

            let red2 = (S::set1_epi32(298) * y2 + S::set1_epi32(409) * v + S::set1_epi32(128)) >> 8;
            let green2 =
                (S::set1_epi32(298) * y2 - S::set1_epi32(100) * u - S::set1_epi32(208) * v
                    + S::set1_epi32(128))
                    >> 8;
            let blue2 =
                (S::set1_epi32(298) * y2 + S::set1_epi32(516) * u + S::set1_epi32(128)) >> 8;

            destination.write_all(&[
                clip::<S>(red2),
                clip::<S>(green2),
                clip::<S>(blue2),
                clip::<S>(red1),
                clip::<S>(green1),
                clip::<S>(blue1),
            ])?;

            pixels = &pixels[S::VI32_WIDTH..];
        }

        Ok(())
    }
);

/// Struct for retrieving images from the NAO camera.
pub struct Camera {
    pix_format: PixFormat,
    camera_stream: ReadStream,
}

impl Camera {
    /// Create a new camera object from a path to the camera device.
    pub fn new_from_path(camera_path: &Path) -> Result<Self> {
        let camera = Device::open(camera_path)?;

        Self::new_from_device(camera)
    }

    /// Create a new camera object from a camera device.
    pub fn new_from_device(camera: Device) -> Result<Self> {
        let requested_pix_format = linuxvideo::format::PixFormat::new(
            NAO_CAMERA_WIDTH,
            NAO_CAMERA_HEIGHT,
            linuxvideo::format::Pixelformat::YUYV,
        );

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

    fn save_rgb_image_from_yuyv(&mut self, mut destination: impl Write) -> Result<()> {
        self.camera_stream.dequeue(|image_buffer_yuv_422| {
            yuyv422_to_rgb2_compiletime(&image_buffer_yuv_422, &mut destination)
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

    /// Save an RGB image as a jpeg.
    ///
    /// The resuling file is a raw stream of bytes, each three bytes representing a single pixel.
    pub fn save_rgb_image_as_jpeg(&mut self, destination: &Path) -> Result<()> {
        let output_file = File::create(destination)?;
        let mut encoder = image::codecs::jpeg::JpegEncoder::new(output_file);

        let mut rgb_buffer = Vec::<u8>::new();
        rgb_buffer.resize((self.image_width() * self.image_height() * 3) as usize, 0);

        match self.pix_format.pixelformat() {
            Pixelformat::YUYV => self.save_rgb_image_from_yuyv(rgb_buffer.as_mut_slice()),
            pixel_format => Ok(eprintln!("Unsupported pixel format: {pixel_format}")),
        }?;

        encoder
            .encode(
                rgb_buffer.as_mut_slice(),
                self.image_width(),
                self.image_height(),
                image::ColorType::Rgb8,
            )
            .unwrap();

        Ok(())
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
