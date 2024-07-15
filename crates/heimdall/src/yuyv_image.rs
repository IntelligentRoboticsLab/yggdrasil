use std::{io::Write, ops::Deref};

use crate::rgb_image::RgbImage;
use crate::Result;

/// An object that holds a YUYV NAO camera image.
pub struct YuyvImage {
    pub(super) frame: linuxvideo::Frame,
    pub(super) width: usize,
    pub(super) height: usize,
}

impl YuyvImage {
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

        // Two pixels are stored in four bytes. Those four bytes are the y1, u, y2, v values in
        // that order. Because two pixels share the same u and v value, we decode both pixels at
        // the same time (using `yuyv422_to_rgb`), instead of one-by-one, to improve performance.
        //
        // A `pixel_duo` here refers to the two pixels with the same u and v values.
        // We iterate over all the pixel duo's in `source`, which is why we take steps of four
        // bytes.
        for pixel_duo_id in 0..(source.len() / 4) {
            let input_offset = pixel_duo_id * 4;

            let y1 = source[input_offset];
            let u = source[input_offset + 1];
            let y2 = source[input_offset + 2];
            let v = source[input_offset + 3];

            let ((red1, green1, blue1), (red2, green2, blue2)) = yuyv422_to_rgb(y1, u, y2, v);

            destination.write_all(&[red1, green1, blue1, red2, green2, blue2])?;
        }

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

    #[must_use]
    pub fn row(&self, index: usize) -> Option<ImageView<'_>> {
        if index >= self.height() {
            panic!("index out of bounds");
        }

        // every 4 bytes stores 2 pixels, so (width / 2) * 4 bytes in a row
        let row_width = self.width();

        let start = index * row_width;
        let end = start + row_width;

        Some(ImageView {
            start,
            end,
            image: self,
        })
    }

    pub fn pixel(&self, x: usize, y: usize) -> Option<YuvPixel> {
        if x >= self.width || y >= self.height {
            return None;
        }

        Some(unsafe { self.pixel_unchecked(x, y) })
    }

    /// Get a pixel at the given coordinates without bounds checking.
    ///
    /// # Safety
    /// Don't be dumb
    pub unsafe fn pixel_unchecked(&self, x: usize, y: usize) -> YuvPixel {
        // every 4 bytes stores 2 pixels, so (width / 2) * 4 bytes in a row
        let offset = (y * self.width + x) * 2;

        let y = if y % 2 == 0 {
            unsafe { *self.frame.get_unchecked(offset) }
        } else {
            unsafe { *self.frame.get_unchecked(offset + 2) }
        };
        let u = unsafe { *self.frame.get_unchecked(offset + 1) };
        let v = unsafe { *self.frame.get_unchecked(offset + 3) };

        YuvPixel { y, u, v }
    }

    pub fn row_iter(&self) -> RowIter<'_> {
        RowIter {
            image: self,
            current_row: 0,
        }
    }

    /// Convert this [`YuyvImage`] to an [`RgbImage`].
    ///
    /// # Errors
    /// This function fails if it cannot allocate an [`RgbImage`].
    pub fn to_rgb(&self) -> Result<RgbImage> {
        let mut rgb_image_buffer = Vec::<u8>::with_capacity(self.width * self.height * 3);
        Self::yuyv_to_rgb(self, &mut rgb_image_buffer)?;

        Ok(RgbImage {
            frame: rgb_image_buffer,
            width: self.width,
            height: self.height,
        })
    }
}

impl Deref for YuyvImage {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.frame[0..self.width * self.height * 2]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YuvPixel {
    pub y: u8,
    pub u: u8,
    pub v: u8,
}

impl YuvPixel {
    pub const BLACK: Self = Self {
        y: 0,
        u: 128,
        v: 128,
    };

    pub fn average(pixels: &[Self]) -> Self {
        let mut y_sum = 0.0;
        let mut u_sum = 0.0;
        let mut v_sum = 0.0;

        for pixel in pixels {
            y_sum += pixel.y as f32;
            u_sum += pixel.u as f32;
            v_sum += pixel.v as f32;
        }

        let len = pixels.len() as f32;

        YuvPixel {
            y: (y_sum / len) as u8,
            u: (u_sum / len) as u8,
            v: (v_sum / len) as u8,
        }
    }

    pub fn to_yhs2(self) -> (f32, f32, f32) {
        let y = self.y as i32;
        let u = self.u as i32;
        let v = self.v as i32;

        let v_normed = v - 128;
        let u_normed = u - 128;

        let h =
            fast_math::atan2(v_normed as f32, u_normed as f32) * std::f32::consts::FRAC_1_PI * 127.
                + 127.;
        let s = (((v_normed.pow(2) + u_normed.pow(2)) * 2) as f32).sqrt() * 255.0 / y as f32;

        (y as f32, h, s)
    }
}

pub struct ImageView<'a> {
    start: usize,
    end: usize,
    image: &'a YuyvImage,
}

impl Iterator for ImageView<'_> {
    type Item = YuvPixel;

    fn next(&mut self) -> Option<Self::Item> {
        if self.start >= self.end {
            return None;
        }

        let offset = self.start * 2;
        self.start += 1;

        let (y, u, v) = unsafe {
            let y = if self.start % 2 == 1 {
                self.image.get_unchecked(offset)
            } else {
                self.image.get_unchecked(offset + 2)
            };
            let u = self.image.get_unchecked(offset + 1);
            let v = self.image.get_unchecked(offset + 3);

            (*y, *u, *v)
        };

        Some(YuvPixel { y, u, v })
    }
}

pub struct RowIter<'a> {
    image: &'a YuyvImage,
    current_row: usize,
}

impl<'a> Iterator for RowIter<'a> {
    type Item = ImageView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_row >= self.image.height() {
            return None;
        }

        let row = self.image.row(self.current_row)?;
        self.current_row += 1;

        Some(row)
    }
}
