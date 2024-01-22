use std::{fs::File, io::Write, ops::Deref};

use image::codecs::jpeg::JpegEncoder;

use crate::rgb_image::RgbImage;
use crate::Result;

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

/// An object that holds a YUYV NAO camera image.
pub struct YuyvImage {
    pub(super) frame: linuxvideo::Frame,
    pub(super) width: u32,
    pub(super) height: u32,
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

    #[must_use]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> u32 {
        self.height
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

    #[must_use]
    pub fn yuv_row_iter(&self) -> YuvRowIter {
        YuvRowIter::new(self)
    }

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

impl<'a> YuvRowIter<'a> {
    pub(crate) fn new(yuyv_image: &'a YuyvImage) -> Self {
        Self {
            yuyv_image,
            current_pos: 0,
            current_rev_pos: (yuyv_image.width * yuyv_image.height) as usize,
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

        Some(if self.current_pos % 2 == 1 {
            YuvPixel {
                y: self.yuyv_image[offset],
                u: self.yuyv_image[offset + 1],
                v: self.yuyv_image[offset + 3],
            }
        } else {
            YuvPixel {
                y: self.yuyv_image[offset + 2],
                u: self.yuyv_image[offset + 1],
                v: self.yuyv_image[offset + 3],
            }
        })
    }
}

impl<'a> DoubleEndedIterator for YuvRowIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.current_pos == self.current_rev_pos {
            return None;
        }

        self.current_rev_pos -= 1;
        let offset = (self.current_rev_pos / 2) * 4;

        Some(if self.current_rev_pos % 2 == 0 {
            YuvPixel {
                y: self.yuyv_image[offset],
                u: self.yuyv_image[offset + 1],
                v: self.yuyv_image[offset + 3],
            }
        } else {
            YuvPixel {
                y: self.yuyv_image[offset + 2],
                u: self.yuyv_image[offset + 1],
                v: self.yuyv_image[offset + 3],
            }
        })
    }
}

pub struct YuvColIter<'a> {
    yuyv_image: &'a YuyvImage,

    current_row: usize,
    current_col: usize,

    current_rev_row: usize,
    current_rev_col: usize,
}

impl<'a> YuvColIter<'a> {
    pub(crate) fn new(yuyv_image: &'a YuyvImage) -> Self {
        Self {
            yuyv_image,
            current_row: 0,
            current_col: 0,
            current_rev_row: yuyv_image.height as usize,
            current_rev_col: yuyv_image.width as usize,
        }
    }
}

impl<'a> Iterator for YuvColIter<'a> {
    type Item = YuvPixel;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_row == self.current_rev_row && self.current_col + 1 == self.current_rev_col
        {
            return None;
        }

        if self.current_row == self.yuyv_image.height as usize {
            self.current_row = 0;
            self.current_col += 1;
        }

        let offset =
            (self.current_row * (self.yuyv_image.width as usize) + self.current_col) / 2 * 4;

        self.current_row += 1;

        Some(if self.current_col % 2 == 0 {
            YuvPixel {
                y: self.yuyv_image[offset],
                u: self.yuyv_image[offset + 1],
                v: self.yuyv_image[offset + 3],
            }
        } else {
            YuvPixel {
                y: self.yuyv_image[offset + 2],
                u: self.yuyv_image[offset + 1],
                v: self.yuyv_image[offset + 3],
            }
        })
    }
}

impl<'a> DoubleEndedIterator for YuvColIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.current_row == self.current_rev_row && self.current_col + 1 == self.current_rev_col
        {
            return None;
        }

        if self.current_rev_row == 0 {
            self.current_rev_col -= 1;

            if self.current_rev_col == 0 {
                return None;
            }

            self.current_rev_row = self.yuyv_image.height as usize;
        }
        self.current_rev_row -= 1;

        let offset =
            (self.current_rev_row * (self.yuyv_image.width as usize) + self.current_rev_col - 1)
                / 2
                * 4;

        Some(if self.current_rev_col % 2 == 1 {
            YuvPixel {
                y: self.yuyv_image[offset],
                u: self.yuyv_image[offset + 1],
                v: self.yuyv_image[offset + 3],
            }
        } else {
            YuvPixel {
                y: self.yuyv_image[offset + 2],
                u: self.yuyv_image[offset + 1],
                v: self.yuyv_image[offset + 3],
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Camera, Result};

    #[test]
    #[ignore]
    fn yuv_row_iter_test() -> Result<()> {
        let mut camera = Camera::new("/dev/video0", 1280, 960, 3)?;
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
        let mut camera = Camera::new("/dev/video0", 1280, 960, 3)?;
        let image = camera.get_yuyv_image()?;

        let mut image_iter = image.yuv_row_iter().rev();
        for row in (0..image.height()).rev() {
            for col in (0..image.width()).rev() {
                let offset = ((row * image.width() + col) / 2) * 4;

                let (y, u, v) = if col % 2 == 0 {
                    (
                        (&*image)[offset as usize],
                        (&*image)[offset as usize + 1],
                        (&*image)[offset as usize + 3],
                    )
                } else {
                    (
                        (&*image)[offset as usize + 2],
                        (&*image)[offset as usize + 1],
                        (&*image)[offset as usize + 3],
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
        let mut camera = Camera::new("/dev/video0", 1280, 960, 3)?;
        let image = camera.get_yuyv_image()?;

        let mut image_iter = image.yuv_col_iter();
        for col in 0..image.width() {
            for row in 0..image.height() {
                let offset = ((row * image.width() + col) / 2) * 4;

                let (y, u, v) = if col % 2 == 0 {
                    (
                        (&*image)[offset as usize],
                        (&*image)[offset as usize + 1],
                        (&*image)[offset as usize + 3],
                    )
                } else {
                    (
                        (&*image)[offset as usize + 2],
                        (&*image)[offset as usize + 1],
                        (&*image)[offset as usize + 3],
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
        let mut camera = Camera::new("/dev/video0", 1280, 960, 3)?;
        let image = camera.get_yuyv_image()?;

        let mut image_iter = image.yuv_col_iter().rev();
        for col in (0..image.width()).rev() {
            for row in (0..image.height()).rev() {
                let offset = ((row * image.width() + col) / 2) * 4;

                let (y, u, v) = if col % 2 == 0 {
                    (
                        (&*image)[offset as usize],
                        (&*image)[offset as usize + 1],
                        (&*image)[offset as usize + 3],
                    )
                } else {
                    (
                        (&*image)[offset as usize + 2],
                        (&*image)[offset as usize + 1],
                        (&*image)[offset as usize + 3],
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
