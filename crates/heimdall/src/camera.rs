use std::{
    fs::File,
    io::{self, Write},
    ops::Deref,
};

use image::codecs::jpeg::JpegEncoder;
use linuxvideo::{format::PixFormat, format::PixelFormat, stream::FrameProvider, Device};

use crate::Result;

/// The width of a NAO [`Image`].
const IMAGE_WIDTH: u32 = 1280;

/// The height of a NAO [`Image`].
const IMAGE_HEIGHT: u32 = 960;

/// Absolute path to the lower camera of the NAO.
const CAMERA_BOTTOM: &str = "/dev/video-bottom";

/// Absolute path to the upper camera of the NAO.
const CAMERA_TOP: &str = "/dev/video-top";

/// An object that holds a YUYV NAO camera image.
pub struct YuyvImage {
    frame: linuxvideo::Frame,
    width: u32,
    height: u32,
}

impl YuyvImage {
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }
}

/// An object that holds a YUYV NAO camera image.
pub struct RgbImage {
    frame: Vec<u8>,
    width: u32,
    height: u32,
}

impl RgbImage {
    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
    }
}

fn yuyv_to_rgb(source: &[u8], mut destination: impl Write) -> Result<()> {
    fn clamp(value: i32) -> u8 {
        #[allow(clippy::cast_sign_loss)]
        #[allow(clippy::cast_possible_truncation)]
        return value.clamp(0, 255) as u8;
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
            (clamp(red1), clamp(green1), clamp(blue1)),
            (clamp(red2), clamp(green2), clamp(blue2)),
        )
    }

    let num_pixels = source.len() / 2;

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
        // Use this if the image should not be flipped.
        // destination.write_all(&[red1, green1, blue1, red2, green2, blue2])?;
    }

    Ok(())
}

impl YuyvImage {
    /// Store the image as a jpeg to a file.
    ///
    /// # Errors
    /// This function fails if it cannot convert the taken image, or if it cannot write to the
    /// file.
    pub fn store_jpeg(&self, file_path: &str) -> Result<()> {
        let output_file = File::create(file_path)?;
        let mut encoder = JpegEncoder::new(output_file);

        let mut rgb_buffer = Vec::<u8>::with_capacity((self.width * self.height * 3) as usize);

        yuyv_to_rgb(self, &mut rgb_buffer)?;

        encoder.encode(&rgb_buffer, self.width, self.height, image::ColorType::Rgb8)?;

        Ok(())
    }

    /// Convert this [`YuyvImage`] to RGB and store it in `destination`.
    ///
    /// # Errors
    /// This function fails if it cannot completely write the RGB image to `destination`.
    pub fn to_rgb(&self) -> Result<RgbImage> {
        let mut rgb_image_buffer =
            Vec::<u8>::with_capacity((self.width * self.height * 3) as usize);
        yuyv_to_rgb(self, &mut rgb_image_buffer)?;

        Ok(RgbImage {
            frame: rgb_image_buffer,
            width: self.width,
            height: self.height,
        })
    }

    pub fn yuv_row_iter(&self) -> YuvRowIter {
        YuvRowIter::new(self)
    }

    pub fn yuv_rev_row_iter(&self) -> YuvRevRowIter {
        YuvRevRowIter::new(self)
    }

    pub fn yuv_col_iter(&self) -> YuvColIter {
        YuvColIter::new(self)
    }

    pub fn yuv_rev_col_iter(&self) -> YuvRevColIter {
        YuvRevColIter::new(self)
    }
}

pub struct YuvRowIter<'a> {
    yuyv_image: &'a YuyvImage,
    current_pos: usize,
}

impl<'a> YuvRowIter<'a> {
    pub(crate) fn new(yuyv_image: &'a YuyvImage) -> Self {
        Self {
            yuyv_image,
            current_pos: 0,
        }
    }
}

impl<'a> Iterator for YuvRowIter<'a> {
    type Item = (u8, u8, u8);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_pos == (self.yuyv_image.width() * self.yuyv_image.height()) as usize {
            return None;
        }

        let offset = (self.current_pos / 2) * 4;
        self.current_pos += 1;

        Some(if self.current_pos % 2 == 1 {
            (
                self.yuyv_image[offset],
                self.yuyv_image[offset + 1],
                self.yuyv_image[offset + 3],
            )
        } else {
            (
                self.yuyv_image[offset + 2],
                self.yuyv_image[offset + 1],
                self.yuyv_image[offset + 3],
            )
        })
    }
}

pub struct YuvRevRowIter<'a> {
    yuyv_image: &'a YuyvImage,
    current_pos: usize,
}

impl<'a> YuvRevRowIter<'a> {
    pub(crate) fn new(yuyv_image: &'a YuyvImage) -> Self {
        Self {
            yuyv_image,
            current_pos: (yuyv_image.width() * yuyv_image.height()) as usize,
        }
    }
}

impl<'a> Iterator for YuvRevRowIter<'a> {
    type Item = (u8, u8, u8);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_pos == 0 {
            return None;
        }

        self.current_pos -= 1;
        let offset = (self.current_pos / 2) * 4;

        Some(if self.current_pos % 2 == 0 {
            (
                self.yuyv_image[offset],
                self.yuyv_image[offset + 1],
                self.yuyv_image[offset + 3],
            )
        } else {
            (
                self.yuyv_image[offset + 2],
                self.yuyv_image[offset + 1],
                self.yuyv_image[offset + 3],
            )
        })
    }
}

impl Deref for YuyvImage {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.frame
    }
}

pub struct YuvColIter<'a> {
    yuyv_image: &'a YuyvImage,

    current_row: usize,
    current_col: usize,
}

impl<'a> YuvColIter<'a> {
    pub(crate) fn new(yuyv_image: &'a YuyvImage) -> Self {
        Self {
            yuyv_image,
            current_row: 0,
            current_col: 0,
        }
    }
}

impl<'a> Iterator for YuvColIter<'a> {
    type Item = (u8, u8, u8);

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_row == self.yuyv_image.height as usize {
            self.current_row = 0;
            self.current_col += 1;
        }

        if self.current_col == self.yuyv_image.width as usize {
            return None;
        }

        let offset =
            (self.current_row * (self.yuyv_image.width() as usize) + self.current_col) / 2 * 4;

        self.current_row += 1;

        Some(if self.current_col % 2 == 0 {
            (
                self.yuyv_image[offset],
                self.yuyv_image[offset + 1],
                self.yuyv_image[offset + 3],
            )
        } else {
            (
                self.yuyv_image[offset + 2],
                self.yuyv_image[offset + 1],
                self.yuyv_image[offset + 3],
            )
        })
    }
}

pub struct YuvRevColIter<'a> {
    yuyv_image: &'a YuyvImage,

    current_row: isize,
    current_col: isize,
}

impl<'a> YuvRevColIter<'a> {
    pub(crate) fn new(yuyv_image: &'a YuyvImage) -> Self {
        Self {
            yuyv_image,
            current_row: yuyv_image.height() as isize,
            current_col: yuyv_image.width() as isize,
        }
    }
}

impl<'a> Iterator for YuvRevColIter<'a> {
    type Item = (u8, u8, u8);

    fn next(&mut self) -> Option<Self::Item> {
        self.current_row -= 1;

        if self.current_row == -1 {
            self.current_col -= 1;

            if self.current_col == -1 {
                return None;
            }

            self.current_row = self.yuyv_image.height() as isize - 1;
        }

        let offset = ((self.current_row * (self.yuyv_image.width() as isize) + self.current_col
            - 1)
            / 2
            * 4) as usize;

        Some(if self.current_col % 2 == 1 {
            (
                self.yuyv_image[offset],
                self.yuyv_image[offset + 1],
                self.yuyv_image[offset + 3],
            )
        } else {
            (
                self.yuyv_image[offset + 2],
                self.yuyv_image[offset + 1],
                self.yuyv_image[offset + 3],
            )
        })
    }
}

impl Deref for RgbImage {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.frame
    }
}

/// Struct for retrieving images from the NAO camera.
pub struct Camera {
    camera: FrameProvider,
    width: u32,
    height: u32,
}

impl Camera {
    /// Create a new camera object.
    ///
    /// # Errors
    /// This function fails if the [`Camera`] cannot be opened.
    pub fn new(device_path: &str, width: u32, height: u32, num_buffers: u32) -> Result<Self> {
        if num_buffers == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Camera must have at least one buffer",
            ))?;
        }

        let capture_device = Device::open(device_path)?.video_capture(PixFormat::new(
            width,
            height,
            PixelFormat::YUYV,
        ))?;
        if capture_device.format().pixel_format() != PixelFormat::YUYV {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Pixel formats other than YUYV are not supported",
            ))?;
        }
        let width = capture_device.format().width();
        let height = capture_device.format().height();

        let camera = capture_device
            .into_stream_num_buffers(num_buffers)?
            .into_frame_provider();

        let mut camera = Self {
            camera,
            width,
            height,
        };

        // Grab some images to make startup the camera.
        // Without it, the first couple of images will return an empty buffer.
        for _ in 0..num_buffers {
            camera.get_yuyv_image()?;
        }

        Ok(camera)
    }

    /// Create a new camera object for the NAO's top camera.
    ///
    /// # Errors
    /// This function fails if the [`Camera`] cannot be opened.
    pub fn new_nao_top(num_buffers: u32) -> Result<Self> {
        Self::new(CAMERA_TOP, IMAGE_WIDTH, IMAGE_HEIGHT, num_buffers)
    }

    /// Create a new camera object for the NAO's bottom camera.
    ///
    /// # Errors
    /// This function fails if the [`Camera`] cannot be opened.
    pub fn new_nao_bottom(num_buffers: u32) -> Result<Self> {
        Self::new(CAMERA_BOTTOM, IMAGE_WIDTH, IMAGE_HEIGHT, num_buffers)
    }

    /// Get the next image.
    ///
    /// # Errors
    /// This function fails if the [`Camera`] cannot take an image.
    pub fn get_yuyv_image(&mut self) -> Result<YuyvImage> {
        let frame = self.camera.fetch_frame()?;

        Ok(YuyvImage {
            frame,
            width: self.width,
            height: self.height,
        })
    }

    /// Get the next image.
    ///
    /// # Errors
    /// This function fails if the [`Camera`] cannot take an image.
    pub fn try_get_yuyv_image(&mut self) -> Result<YuyvImage> {
        let frame = self.camera.try_fetch_frame()?;

        Ok(YuyvImage {
            frame,
            width: self.width,
            height: self.height,
        })
    }
}
