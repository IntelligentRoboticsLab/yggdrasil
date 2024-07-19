//! Utility functions for machine learning.

use fast_image_resize as fr;
use std::num::NonZeroU32;

/// Returns the index of the maximum element in a [`Vec`].
pub fn argmax(v: &[f32]) -> usize {
    v.iter()
        .enumerate()
        .max_by(|(_, v1), (_, v2)| v1.total_cmp(v2))
        .expect("argmax: empty vector")
        .0
}

/// Returns the softmax of [`Vec`].
pub fn softmax(v: &[f32]) -> Vec<f32> {
    let exps = v.iter().map(|f| f.exp()).collect::<Vec<_>>();

    let sum: f32 = exps.iter().sum();
    exps.iter().map(|x| x / sum).collect()
}

/// Computes the sigmoid score of the provided logit.
pub fn sigmoid(logit: f32) -> f32 {
    1.0 / (1.0 + (-logit).exp())
}

pub fn resize_patch(original: (usize, usize), target: (usize, usize), patch: Vec<u8>) -> Vec<f32> {
    let src_image = fr::Image::from_vec_u8(
        NonZeroU32::new(original.0 as u32).unwrap(),
        NonZeroU32::new(original.1 as u32).unwrap(),
        patch,
        fr::PixelType::U8,
    )
    .expect("Failed to create image for resizing");

    // Resize the image to the correct input shape for the model
    let mut dst_image = fr::Image::new(
        NonZeroU32::new(target.0 as u32).unwrap(),
        NonZeroU32::new(target.1 as u32).unwrap(),
        src_image.pixel_type(),
    );

    let mut resizer = fr::Resizer::new(fr::ResizeAlg::Nearest);
    resizer
        .resize(&src_image.view(), &mut dst_image.view_mut())
        .expect("Failed to resize image");

    dst_image
        .buffer()
        .iter()
        .map(|x| *x as f32 / 255.0)
        .collect()
}
