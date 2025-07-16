//! Utility functions for machine learning.

use fast_image_resize::{self as fir, ResizeOptions, Resizer, images::Image};

/// Returns the index of the maximum element in a [`Vec`].
///
/// # Panics
///
/// If the input vector is empty this function will panic.
#[inline]
#[must_use]
pub fn argmax(v: &[f32]) -> usize {
    let mut max_index = 0;
    let mut max_value = v[0];

    for (i, &value) in v.iter().enumerate().skip(1) {
        if value > max_value {
            max_index = i;
            max_value = value;
        }
    }

    max_index
}

/// Returns the softmax of [`Vec`].
#[inline]
#[must_use]
pub fn softmax(v: &[f32]) -> Vec<f32> {
    let exps = v.iter().map(|f| f.exp()).collect::<Vec<_>>();

    let sum: f32 = exps.iter().sum();
    exps.iter().map(|x| x / sum).collect()
}

/// Computes the sigmoid score of the provided logit.
#[inline]
#[must_use]
pub fn sigmoid(logit: f32) -> f32 {
    1.0 / (1.0 + (-logit).exp())
}

/// Helper utility to resize patches without copying data.
pub struct PatchResizer {
    resizer: Resizer,
    dst_image: Image<'static>,
}

impl PatchResizer {
    /// Create a new [`PatchResizer`] with the target dimensions.
    #[must_use]
    pub fn new(target_w: u32, target_h: u32) -> Self {
        let dst_image = Image::new(target_w, target_h, fir::PixelType::U8);

        #[cfg(not(target_arch = "x86_64"))]
        {
            let resizer = Resizer::new();
            Self { resizer, dst_image }
        }

        #[cfg(target_arch = "x86_64")]
        {
            let mut resizer = Resizer::new();

            if fir::CpuExtensions::Sse4_1.is_supported() {
                unsafe {
                    resizer.set_cpu_extensions(fir::CpuExtensions::Sse4_1);
                }
            }
            Self { resizer, dst_image }
        }
    }

    /// Resize a grayscale patch (borrowed from the parent frame) into the
    /// internal dst buffer. Returns the resized bytes (borrowed).
    ///
    /// # Panics
    ///
    /// This function panics if the source dimensions are zero or if the
    /// patch is empty.
    #[inline]
    pub fn resize_patch<'a>(
        &'a mut self,
        src_frame: &'a [u8],
        src_dims: (usize, usize),
    ) -> &'a [u8] {
        let src_image = fir::images::ImageRef::new(
            src_dims.0 as u32,
            src_dims.1 as u32,
            src_frame,
            fir::PixelType::U8,
        )
        .expect("invalid source dims");

        let crop_opts = ResizeOptions::new().resize_alg(fir::ResizeAlg::Nearest);

        self.resizer
            .resize(&src_image, &mut self.dst_image, &crop_opts)
            .expect("resize failed");

        self.dst_image.buffer()
    }

    /// Take the resized buffer and return it as a [`Vec<u8>`].
    pub fn take(&mut self) -> Vec<u8> {
        let new_img = Image::new(
            self.dst_image.width(),
            self.dst_image.height(),
            self.dst_image.pixel_type(),
        );
        let old = std::mem::replace(&mut self.dst_image, new_img);

        // moves buffer without a copy, because old is owned.
        old.into_vec()
    }
}
