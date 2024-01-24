use std::path::Path;
use std::{fs::File, io::Write, ops::Deref};

use image::codecs::jpeg::JpegEncoder;

use crate::rgb_image::RgbImage;
use crate::Result;

/// An object that holds a YUYV NAO camera image.
pub struct YuyvImage {
    pub(super) frame: linuxvideo::Frame,
    pub(super) width: usize,
    pub(super) height: usize,
}

impl YuyvImage {
    fn yuyv_to_rgb(
        source: &[u8],
        mut destination: impl Write,
        rotate_180_degrees: bool,
    ) -> Result<()> {
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

        // Two pixels are stored in four bytes. Those four bytes are the y1, u, y2, v values in
        // that order. Because two pixels share the same u and v value, we decode both pixels at
        // the same time (using `yuyv422_to_rgb`), instead of one-by-one, to improve performance.
        //
        // A `pixel_duo` here refers to the two pixels with the same u and v values.
        // We iterate over all the pixel duo's in `source`, which is why we take steps of four
        // bytes.
        for pixel_duo_id in 0..(source.len() / 4) {
            let input_offset: usize = if rotate_180_degrees {
                source.len() - 4 * pixel_duo_id - 4
            } else {
                pixel_duo_id * 4
            };

            let y1 = source[input_offset];
            let u = source[input_offset + 1];
            let y2 = source[input_offset + 2];
            let v = source[input_offset + 3];

            let ((red1, green1, blue1), (red2, green2, blue2)) = yuyv422_to_rgb(y1, u, y2, v);

            if rotate_180_degrees {
                destination.write_all(&[red2, green2, blue2, red1, green1, blue1])?;
            } else {
                destination.write_all(&[red1, green1, blue1, red2, green2, blue2])?;
            }
        }

        Ok(())
    }

    /// Store the image as a jpeg to a file.
    ///
    /// The image is rotated 180 degrees.
    ///
    /// # Errors
    /// This function fails if it cannot convert the taken image, or if it cannot write to the
    /// file.
    ///
    /// # Panics
    /// This function pannics if it cannot convert a `u32` value to `usize`.
    pub fn store_jpeg(&self, file_path: impl AsRef<Path>) -> Result<()> {
        let output_file = File::create(file_path)?;
        let mut encoder = JpegEncoder::new(output_file);

        let mut rgb_buffer = Vec::<u8>::with_capacity(self.width * self.height * 3);

        Self::yuyv_to_rgb(self, &mut rgb_buffer, true)?;

        encoder.encode(
            &rgb_buffer,
            u32::try_from(self.width).unwrap(),
            u32::try_from(self.height).unwrap(),
            image::ColorType::Rgb8,
        )?;

        Ok(())
    }

    #[must_use]
    pub fn width(&self) -> usize {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> usize {
        self.height
    }

    /// Convert this [`YuyvImage`] to an [`RgbImage`].
    ///
    /// # Errors
    /// This function fails if it cannot allocate an [`RgbImage`].
    pub fn to_rgb(&self) -> Result<RgbImage> {
        let mut rgb_image_buffer = Vec::<u8>::with_capacity(self.width * self.height * 3);
        Self::yuyv_to_rgb(self, &mut rgb_image_buffer, false)?;

        Ok(RgbImage {
            frame: rgb_image_buffer,
            width: self.width,
            height: self.height,
        })
    }

    /// Return a row-iterator over the image.
    ///
    /// This iterator iterates over the image, row by row, from left to right column, starting at the
    /// top row.
    #[must_use]
    pub fn yuv_row_iter(&self) -> YuvRowIter {
        YuvRowIter::new(self)
    }

    /// Return a column-iterator over the image.
    ///
    /// This iterator iterates over the image, column by column, from the top row to the bottom row, starting at the
    /// most left column.
    #[must_use]
    pub fn yuv_col_iter(&self) -> YuvColIter {
        YuvColIter::new(self)
    }
}

impl Deref for YuyvImage {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.frame
    }
}

pub struct YuvPixel {
    pub y: u8,
    pub u: u8,
    pub v: u8,
}

pub struct YuvRowIter<'a> {
    yuyv_image: &'a YuyvImage,
    current_pos: usize,
    current_rev_pos: usize,
}

/// A row-iterator over a [`YuyvImage`].
///
/// This iterator iterates over the image, row by row, from left to right column, starting at the
/// top row.
impl<'a> YuvRowIter<'a> {
    pub(crate) fn new(yuyv_image: &'a YuyvImage) -> Self {
        Self {
            yuyv_image,
            current_pos: 0,
            current_rev_pos: yuyv_image.width * yuyv_image.height,
        }
    }
}

impl<'a> Iterator for YuvRowIter<'a> {
    type Item = YuvPixel;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_pos == self.current_rev_pos {
            return None;
        }

        let offset = (self.current_pos / 2) * 4;
        self.current_pos += 1;

        let y = if self.current_pos % 2 == 1 {
            self.yuyv_image[offset]
        } else {
            self.yuyv_image[offset + 2]
        };
        let u = self.yuyv_image[offset + 1];
        let v = self.yuyv_image[offset + 3];

        Some(YuvPixel { y, u, v })
    }
}

impl<'a> DoubleEndedIterator for YuvRowIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.current_pos == self.current_rev_pos {
            return None;
        }

        self.current_rev_pos -= 1;
        let offset = (self.current_rev_pos / 2) * 4;

        let y = if self.current_rev_pos % 2 == 0 {
            self.yuyv_image[offset]
        } else {
            self.yuyv_image[offset + 2]
        };
        let u = self.yuyv_image[offset + 1];
        let v = self.yuyv_image[offset + 3];

        Some(YuvPixel { y, u, v })
    }
}

/// A column-iterator over a [`YuyvImage`].
///
/// This iterator iterates over the image, column by column, from the top row to the bottom row, starting at the
/// most left column.
pub struct YuvColIter<'a> {
    yuyv_image: &'a YuyvImage,

    current_pos: usize,
    current_rev_pos: usize,
}

impl<'a> YuvColIter<'a> {
    pub(crate) fn new(yuyv_image: &'a YuyvImage) -> Self {
        Self {
            yuyv_image,
            current_pos: 0,
            current_rev_pos: yuyv_image.width * yuyv_image.height,
        }
    }
}

impl<'a> Iterator for YuvColIter<'a> {
    type Item = YuvPixel;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_pos == self.current_rev_pos {
            return None;
        }

        let col = self.current_pos / self.yuyv_image.height;
        let row = self.current_pos % self.yuyv_image.height;

        let offset = (row * self.yuyv_image.width + col) / 2 * 4;

        self.current_pos += 1;

        let y = if col % 2 == 0 {
            self.yuyv_image[offset]
        } else {
            self.yuyv_image[offset + 2]
        };
        let u = self.yuyv_image[offset + 1];
        let v = self.yuyv_image[offset + 3];

        Some(YuvPixel { y, u, v })
    }
}

impl<'a> DoubleEndedIterator for YuvColIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.current_pos == self.current_rev_pos {
            return None;
        }

        self.current_rev_pos -= 1;

        let col = self.current_rev_pos / self.yuyv_image.height;
        let row = self.current_rev_pos % self.yuyv_image.height;

        let offset = (row * self.yuyv_image.width + col) / 2 * 4;

        let y = if col % 2 == 0 {
            self.yuyv_image[offset]
        } else {
            self.yuyv_image[offset + 2]
        };
        let u = self.yuyv_image[offset + 1];
        let v = self.yuyv_image[offset + 3];

        Some(YuvPixel { y, u, v })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Camera, Result};

    const CAMERA_PATH: &str = "/dev/video0";
    const CAMERA_WIDTH: u32 = 1280;
    const CAMERA_HEIGHT: u32 = 960;
    const NUM_BUFFERS: u32 = 1;

    #[test]
    #[ignore]
    fn yuv_row_iter_test() -> Result<()> {
        let mut camera = Camera::new(CAMERA_PATH, CAMERA_WIDTH, CAMERA_HEIGHT, NUM_BUFFERS)?;
        let image = camera.get_yuyv_image()?;

        let mut num: usize = 0;
        let mut image_iter = image.yuv_row_iter();

        image.iter().as_slice().chunks_exact(4).for_each(|yuyv| {
            num += 1;

            let y1: u8 = yuyv[0];
            let u: u8 = yuyv[1];
            let y2: u8 = yuyv[2];
            let v: u8 = yuyv[3];

            let yuv_pixel = image_iter.next().unwrap();
            assert_eq!(y1, yuv_pixel.y);
            assert_eq!(u, yuv_pixel.u);
            assert_eq!(v, yuv_pixel.v);

            let yuv_pixel = image_iter.next().unwrap();
            assert_eq!(y2, yuv_pixel.y);
            assert_eq!(u, yuv_pixel.u);
            assert_eq!(v, yuv_pixel.v);
        });
        assert!(image_iter.next().is_none());

        Ok(())
    }

    #[test]
    #[ignore]
    fn yuv_rev_row_iter_test() -> Result<()> {
        let mut camera = Camera::new(CAMERA_PATH, CAMERA_WIDTH, CAMERA_HEIGHT, NUM_BUFFERS)?;
        let image = camera.get_yuyv_image()?;

        let mut image_iter = image.yuv_row_iter().rev();
        for row in (0..image.height()).rev() {
            for col in (0..image.width()).rev() {
                let offset = ((row * image.width() + col) / 2) * 4;

                let (y, u, v) = if col % 2 == 0 {
                    (
                        (&*image)[offset],
                        (&*image)[offset + 1],
                        (&*image)[offset + 3],
                    )
                } else {
                    (
                        (&*image)[offset + 2],
                        (&*image)[offset + 1],
                        (&*image)[offset + 3],
                    )
                };

                let yuv_pixel = image_iter.next().unwrap();

                assert_eq!(y, yuv_pixel.y);
                assert_eq!(u, yuv_pixel.u);
                assert_eq!(v, yuv_pixel.v);
            }
        }
        assert!(image_iter.next().is_none());

        Ok(())
    }

    #[test]
    #[ignore]
    fn yuv_col_iter_test() -> Result<()> {
        let mut camera = Camera::new(CAMERA_PATH, CAMERA_WIDTH, CAMERA_HEIGHT, NUM_BUFFERS)?;
        let image = camera.get_yuyv_image()?;

        let mut image_iter = image.yuv_col_iter();
        for col in 0..image.width() {
            for row in 0..image.height() {
                let offset = ((row * image.width() + col) / 2) * 4;

                let (y, u, v) = if col % 2 == 0 {
                    (
                        (&*image)[offset],
                        (&*image)[offset + 1],
                        (&*image)[offset + 3],
                    )
                } else {
                    (
                        (&*image)[offset + 2],
                        (&*image)[offset + 1],
                        (&*image)[offset + 3],
                    )
                };

                let yuv_pixel = image_iter.next().unwrap();

                assert_eq!(y, yuv_pixel.y);
                assert_eq!(u, yuv_pixel.u);
                assert_eq!(v, yuv_pixel.v);
            }
        }
        assert!(image_iter.next().is_none());

        Ok(())
    }

    #[test]
    #[ignore]
    fn yuv_rev_col_iter_test() -> Result<()> {
        let mut camera = Camera::new(CAMERA_PATH, CAMERA_WIDTH, CAMERA_HEIGHT, NUM_BUFFERS)?;
        let image = camera.get_yuyv_image()?;

        let mut image_iter = image.yuv_col_iter().rev();
        for col in (0..image.width()).rev() {
            for row in (0..image.height()).rev() {
                let offset = ((row * image.width() + col) / 2) * 4;

                let (y, u, v) = if col % 2 == 0 {
                    (
                        (&*image)[offset],
                        (&*image)[offset + 1],
                        (&*image)[offset + 3],
                    )
                } else {
                    (
                        (&*image)[offset + 2],
                        (&*image)[offset + 1],
                        (&*image)[offset + 3],
                    )
                };

                let yuv_pixel = image_iter.next().unwrap();

                assert_eq!(y, yuv_pixel.y);
                assert_eq!(u, yuv_pixel.u);
                assert_eq!(v, yuv_pixel.v);
            }
        }
        assert!(image_iter.next().is_none());

        Ok(())
    }
}
