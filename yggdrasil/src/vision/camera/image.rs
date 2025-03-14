use bevy::prelude::*;
use heimdall::{CameraLocation, YuyvImage};
use std::{marker::PhantomData, sync::Arc, time::Instant};

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

    #[must_use]
    pub fn is_from_cycle(&self, cycle: Cycle) -> bool {
        self.cycle == cycle
    }

    #[must_use]
    pub fn yuyv_image(&self) -> &YuyvImage {
        &self.buf
    }

    #[must_use]
    pub fn timestamp(&self) -> Instant {
        self.timestamp
    }

    #[must_use]
    pub fn cycle(&self) -> Cycle {
        self.cycle
    }

    /// Get a grayscale patch from the image centered at the given point.
    /// The patch is of size `width` x `height`, and padded with zeros if the patch goes out of bounds.
    ///
    /// The grayscale values are normalized to the range [0, 1].
    #[must_use]
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

    /// Crops a YUYV patch from the image centered at `center` with dimensions
    /// `width` x `height`. The output is in YUYV format (4 bytes for each pair of pixels).
    ///
    /// The crop assumes the full image is stored in YUYV format (each row has `image.width()`
    /// pixels, and the underlying data is arranged in groups of 4 bytes for every 2 pixels).
    /// If the requested patch goes out of bounds, the missing parts are padded with zeros.
    ///
    /// # Note
    ///
    ///  `width` must be even.
    #[must_use]
    pub fn get_yuyv_patch(&self, center: (usize, usize), width: usize, height: usize) -> Vec<u8> {
        assert!(width % 2 == 0, "Width must be even for YUYV format.");

        let (cx, cy) = center;
        let src = self.yuyv_image();
        let src_width = src.width(); // in pixels
        let src_height = src.height(); // in pixels
                                       // Each pixel is effectively 2 bytes, so reserve space accordingly.
        let mut result = Vec::with_capacity(width * height * 2);

        // Compute the top-left corner of the patch.
        // If the computed starting x is odd, adjust to the previous even number.
        let mut x0 = if cx >= width / 2 { cx - width / 2 } else { 0 };
        if x0 % 2 != 0 {
            x0 = x0.saturating_sub(1);
        }
        let y0 = if cy >= height / 2 { cy - height / 2 } else { 0 };

        // Process each row of the patch.
        for i in 0..height {
            let y = y0 + i;
            if y >= src_height {
                // Out-of-bound row: pad with zeros.
                result.extend(vec![0; width * 2]);
                continue;
            }
            // Process the row in groups of 2 pixels.
            for group in 0..(width / 2) {
                let x = x0 + group * 2;
                if x + 1 < src_width {
                    // Both pixels are in-bound.
                    // In YUYV, each pair is stored as 4 bytes.
                    // Compute the group index: each row has (src_width/2) groups.
                    let group_index = y * (src_width / 2) + (x / 2);
                    let base = group_index * 4;
                    result.extend_from_slice(&src[base..base + 4]);
                } else if x < src_width {
                    // Only one pixel is available (this case might occur if the image width is odd).
                    // Copy the available pixel (2 bytes) and pad the missing one.
                    let group_index = y * (src_width / 2) + (x / 2);
                    let base = group_index * 4;
                    result.push(src[base]); // Y value of the available pixel
                    result.push(src[base + 1]); // U value (shared)
                    result.push(0); // Pad Y for missing pixel
                    result.push(0); // Pad V for missing pixel
                } else {
                    // Both pixels are out-of-bound; pad with zeros.
                    result.extend_from_slice(&[0, 0, 0, 0]);
                }
            }
        }

        result
    }
}
