use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use std::path::Path;
use turbojpeg::OwnedBuf;

use crate::YuyvImage;
use crate::{Error, Result};

pub struct YuvPlanarImage {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

impl YuvPlanarImage {
    pub fn from_yuyv(yuyv_image: &YuyvImage) -> Self {
        let num_pixels = yuyv_image.height() * yuyv_image.width();
        let mut data = vec![0u8; num_pixels * 2];

        for pixel_duo_id in 0..num_pixels / 2 {
            let offset_image = pixel_duo_id * 4;

            let offset_y = pixel_duo_id * 2;
            let offset_u = num_pixels + pixel_duo_id;
            let offset_v = num_pixels + num_pixels / 2 + pixel_duo_id;

            unsafe {
                *data.get_unchecked_mut(offset_y) = *yuyv_image.get_unchecked(offset_image);
                *data.get_unchecked_mut(offset_y + 1) = *yuyv_image.get_unchecked(offset_image + 2);

                *data.get_unchecked_mut(offset_u) = *yuyv_image.get_unchecked(offset_image + 1);
                *data.get_unchecked_mut(offset_v) = *yuyv_image.get_unchecked(offset_image + 3);
            };
        }

        Self {
            width: yuyv_image.width(),
            height: yuyv_image.height(),
            data,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// Convert this [`YuvPlanarImage`] to a JPEG image.
    ///
    /// The quality of the JPEG image is determined by the `quality` parameter. The value should be
    /// between 1 and 100, where 1 is the worst quality and 100 is the best quality.
    ///
    /// # Errors
    /// This function fails if it cannot convert the taken image.
    pub fn to_jpeg(&self, quality: i32) -> Result<OwnedBuf> {
        let img = turbojpeg::YuvImage {
            pixels: self.deref(),
            width: self.width(),
            align: 2,
            height: self.height(),
            subsamp: turbojpeg::Subsamp::Sub2x1,
        };

        turbojpeg::compress_yuv(img, quality).map_err(Error::Jpeg)
    }

    /// Store the image as a jpeg to a file.
    ///
    /// # Errors
    /// This function fails if it cannot convert the taken image, or if it cannot write to the
    /// file.
    ///
    /// # Panics
    /// This function pannics if it cannot convert a `u32` value to `usize`.
    pub fn store_jpeg(&self, file_path: impl AsRef<Path>, quality: i32) -> Result<()> {
        let mut output_file = File::create(file_path)?;
        let jpeg = self.to_jpeg(quality)?;
        output_file.write_all(&jpeg)?;

        Ok(())
    }
}

impl Deref for YuvPlanarImage {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.data
    }
}
