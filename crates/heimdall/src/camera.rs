//! Contains functions for simplifying the use of a V4L2
//!
//! Written for the Dutch Nao Team as part of project Yggdrasil
//! <https://github.com/intelligentroboticslab>
//!
//! by O. Bosgraaf
use std::{
    ffi::OsStr,
    fs::File,
    io::{BufWriter, Write},
    path::Path,
};

use crate::{error::Error, Result};

/// The camera width of a NAO v6.
const NAO_CAMERA_WIDTH: u32 = 1280;

/// The camera height of a NAO v6.
const NAO_CAMERA_HEIGHT: u32 = 960;

use linuxvideo::{
    format::{PixFormat, Pixelformat},
    stream::ReadStream,
    Device,
};

use core::{arch::x86_64::*, slice::SlicePattern};

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
            let four_bytes = S::loadu_epi32(&*(pixels.as_ptr() as *const i32));

            let y1 = four_bytes;
            let u = y1 >> 8;
            let y2 = u >> 8;
            let v = y2 >> 8;

            let y1 = y1 & S::set1_epi32(255) - S::set1_epi32(16);
            let u = u & S::set1_epi32(255) - S::set1_epi32(128);
            let y2 = y2 & S::set1_epi32(255) - S::set1_epi32(16);
            let v = v - S::set1_epi32(128);

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

    unsafe fn yuyv_to_rgb(pixels: &[u8], destination: &mut impl Write) -> std::io::Result<()> {
        unsafe fn mul_i32(a: __m128i, b: __m128i) -> __m128i {
            // let mut a_buf = [0i32; 4];
            // let mut b_buf = [0i32; 4];
            //
            // _mm_storeu_si128(a_buf.as_mut_ptr() as *mut __m128i, a);
            // _mm_storeu_si128(b_buf.as_mut_ptr() as *mut __m128i, b);
            //
            // for i in 0..4 {
            //     a_buf[i] = a_buf[i] * b_buf[i];
            // }
            //
            // _mm_loadu_si128(a_buf.as_mut_ptr() as *mut __m128i)
            let tmp1 = _mm_mul_epu32(a, b); /* mul 2,0*/
            let tmp2 = _mm_mul_epu32(_mm_srli_si128(a, 4), _mm_srli_si128(b, 4)); /* mul 3,1 */
            return _mm_unpacklo_epi32(
                _mm_shuffle_epi32(tmp1, _MM_SHUFFLE(0, 0, 2, 0)),
                _mm_shuffle_epi32(tmp2, _MM_SHUFFLE(0, 0, 2, 0)),
            );
        }

        unsafe fn clip(a: __m128i) -> __m128i {
            _mm_max_epi32(_mm_min_epi32(a, _mm_set1_epi32(255)), _mm_set1_epi32(0))
        }

        let val_16 = _mm_set1_epi32(16);
        let val_128 = _mm_set1_epi32(128);
        let val_mul_c = _mm_set_epi32(298, 298, 298, 298);
        let val_mul_d = _mm_set_epi32(0, 516, -100, 0);
        let val_mul_e = _mm_set_epi32(0, 0, -208, 409);

        for offset in 0..pixels.len() / 4 {
            let y1 = _mm_set1_epi32(pixels[offset * 4 + 0] as i32);
            let u = _mm_set1_epi32(pixels[offset * 4 + 1] as i32);
            let y2 = _mm_set1_epi32(pixels[offset * 4 + 2] as i32);
            let v = _mm_set1_epi32(pixels[offset * 4 + 3] as i32);

            let c1 = _mm_sub_epi32(y1, val_16);
            let c2 = _mm_sub_epi32(y2, val_16);
            let d = _mm_sub_epi32(u, val_128);
            let e = _mm_sub_epi32(v, val_128);

            let tmp_c1 = mul_i32(c1, val_mul_c);
            let tmp_c2 = mul_i32(c2, val_mul_c);
            let tmp_d = mul_i32(d, val_mul_d);
            let tmp_e = mul_i32(e, val_mul_e);

            let rgb = _mm_add_epi32(val_128, tmp_e);
            let rgb = _mm_add_epi32(rgb, tmp_d);

            let rgb1 = _mm_add_epi32(rgb, tmp_c1);
            let rgb1 = _mm_srai_epi32(rgb1, 8);
            let rgb1 = clip(rgb1);

            let rgb2 = _mm_add_epi32(rgb, tmp_c2);
            let rgb2 = _mm_srai_epi32(rgb2, 8);
            let rgb2 = clip(rgb2);

            destination.write_all(&[
                _mm_extract_epi32(rgb1, 0) as u8,
                _mm_extract_epi32(rgb1, 1) as u8,
                _mm_extract_epi32(rgb1, 2) as u8,
                _mm_extract_epi32(rgb2, 0) as u8,
                _mm_extract_epi32(rgb2, 1) as u8,
                _mm_extract_epi32(rgb2, 2) as u8,
            ])?;
        }
        Ok(())
    }

    fn save_rgb_image_from_yuyv(&mut self, mut destination: impl Write) -> Result<()> {
        self.camera_stream.dequeue(|image_buffer_yuv_422| {
            let num_pixels = (self.pix_format.width() * self.pix_format.height()) as usize;
            let begin = std::time::Instant::now();
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

            eprintln!("serial elapsed: {}", begin.elapsed().as_millis());

            let begin = std::time::Instant::now();
            yuyv422_to_rgb2_compiletime(&image_buffer_yuv_422, &mut destination)?;
            eprintln!("simd elapsed  : {}", begin.elapsed().as_millis());

            let begin = std::time::Instant::now();
            unsafe {
                Self::yuyv_to_rgb(&image_buffer_yuv_422, &mut destination)?;
            }
            eprintln!("intrinsics    : {}", begin.elapsed().as_millis());

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

    /// Save an RGB image as a jpeg.
    ///
    /// The resuling file is a raw stream of bytes, each three bytes representing a single pixel.
    pub fn save_rgb_image_as_jpeg(&mut self, destination: &Path) -> Result<()> {
        let output_file = File::create(destination)?;
        let mut encoder = image::codecs::jpeg::JpegEncoder::new(output_file);

        let mut rgb_buffer =
            Vec::<u8>::with_capacity((self.image_width() * self.image_height() * 3 * 3) as usize);
        rgb_buffer.resize(
            (self.image_height() * self.image_width() * 3 * 3) as usize,
            0,
        );

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

    pub fn save_yuyv_image_to_file<P: AsRef<Path>>(&mut self, destination: P) -> Result<()> {
        let path = destination.as_ref();
        let mut output_file = File::create(path)?;
        eprintln!("size: {}, {}", self.image_width(), self.image_height());

        match self.pix_format.pixelformat() {
            Pixelformat::YUYV => self.camera_stream.dequeue(|image_buffer_yuv_422| {     
                let buf: Vec<_> = image_buffer_yuv_422.chunks(4).into_iter().rev().flatten().cloned().collect();
                output_file.write_all(buf.as_slice())?;
                Ok(())
            }),
            _ => unimplemented!("lmao"),
        }?;

        Ok(())
    }
}
