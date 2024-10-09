use bevy::prelude::*;
use fast_image_resize as fr;
use heimdall::{CameraLocation, YuyvImage};
use miette::{Context, IntoDiagnostic, Result};
use std::{marker::PhantomData, num::NonZeroU32, sync::Arc, time::Instant};

use crate::nao::Cycle;

#[derive(Resource, Deref)]
pub struct Image<T: CameraLocation> {
    #[deref]
    /// Captured image in yuyv format.
    buf: Arc<YuyvImage>,
    /// Instant at which the image was captured.
    timestamp: Instant,
    /// Return the cycle at which the image was captured.
    cycle: Cycle,
    _marker: PhantomData<T>,
}

// NOTE: This needs to be implemented manually because of the `PhantomData`
// https://github.com/rust-lang/rust/issues/26925
impl<T: CameraLocation> Clone for Image<T> {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
            timestamp: self.timestamp,
            cycle: self.cycle,
            _marker: PhantomData,
        }
    }
}

impl<T: CameraLocation> Image<T> {
    pub(super) fn new(yuyv_image: YuyvImage, cycle: Cycle) -> Self {
        Self {
            buf: Arc::new(yuyv_image),
            timestamp: Instant::now(),
            cycle,
            _marker: PhantomData,
        }
    }

    pub fn is_from_cycle(&self, cycle: Cycle) -> bool {
        self.cycle == cycle
    }

    pub fn yuyv_image(&self) -> &YuyvImage {
        &self.buf
    }

    pub fn timestamp(&self) -> Instant {
        self.timestamp
    }

    pub fn cycle(&self) -> Cycle {
        self.cycle
    }

    /// Resizes the image to the given width and height using the specified algorithm.
    ///
    /// The resized image is returned as a vector of bytes, in packed YUV format.
    /// The image is converted to YUV by dropping the second y component of the YUYV format.
    pub fn resized_yuv(
        &self,
        width: u32,
        height: u32,
        algorithm: fr::ResizeAlg,
    ) -> Result<Vec<u8>> {
        let image = self.yuyv_image();

        let src_image = fr::Image::from_vec_u8(
            NonZeroU32::new((image.width() / 2) as u32).unwrap(),
            NonZeroU32::new(image.height() as u32).unwrap(),
            image.to_vec(),
            fr::PixelType::U8x4,
        )
        .into_diagnostic()
        .context("Failed to create source image for resizing!")?;

        let mut dst_image = fr::Image::new(
            NonZeroU32::new(width).unwrap(),
            NonZeroU32::new(height).unwrap(),
            src_image.pixel_type(),
        );

        let mut resizer = fr::Resizer::new(algorithm);

        resizer
            .resize(&src_image.view(), &mut dst_image.view_mut())
            .into_diagnostic()
            .context("Failed to resize image")?;

        // Remove every second y value from the yuyv image to turn it into a packed yuv image
        Ok(dst_image
            .into_vec()
            .into_iter()
            .enumerate()
            .filter(|(i, _)| (i + 2) % 4 != 0)
            .map(|(_, p)| p)
            .collect())
    }

    /// Get a grayscale patch from the image centered at the given point.
    /// The patch is of size `width` x `height`, and padded with zeros if the patch goes out of bounds.
    ///
    /// The grayscale values are normalized to the range [0, 1].
    pub fn get_grayscale_patch(
        &self,
        center: (usize, usize),
        width: usize,
        height: usize,
    ) -> Vec<u8> {
        let (cx, cy) = center;

        let yuyv_image = self.yuyv_image();
        let mut result = Vec::with_capacity(width * height);

        for i in 0..height {
            for j in 0..width {
                let x = cx + j - width / 2;
                let y = cy + i - height / 2;

                if x >= self.yuyv_image().width() || y >= self.yuyv_image().height() {
                    result.push(0);
                    continue;
                }

                let index = y * yuyv_image.width() + x;
                result.push(yuyv_image[index * 2]);
            }
        }

        result
    }
}
