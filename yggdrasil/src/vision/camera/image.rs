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
}
