pub mod bbox;

use miette::{IntoDiagnostic, Result};

use bbox::{ConvertBbox, Xyxy};
use fast_image_resize::{self as fir, ResizeOptions};
use itertools::Itertools;

/// Applies Non-Maximum Suppression (NMS) to the given bounding boxes and scores.
///
/// NMS is used to remove overlapping boxes with lower scores, keeping only the highest scoring
/// boxes.
pub fn non_max_suppression<B>(detections: &[(B, f32)], threshold: f32) -> Vec<usize>
where
    B: ConvertBbox<Xyxy> + Clone + Copy,
{
    let mut final_indices = Vec::new();

    for i in 0..detections.len() {
        let mut discard = false;
        for j in 0..detections.len() {
            if i == j {
                continue;
            }

            let (box_i, score_i) = detections[i];
            let (box_j, score_j) = detections[j];

            let iou = box_i.convert().iou(&box_j);
            if iou >= threshold && score_j >= score_i {
                discard = true;
                break;
            }
        }

        if !discard {
            final_indices.push(i);
        }
    }

    final_indices
}

/// Resizes a raw buffer of yuyv data.
pub fn resize_image(
    image: Vec<u8>,
    image_width: u32,
    image_height: u32,
    target_width: u32,
    target_height: u32,
) -> Result<Vec<u8>> {
    assert!(target_width % 2 == 0, "width must be a multiple of 2");

    let src_image =
        fir::images::Image::from_vec_u8(image_width / 2, image_height, image, fir::PixelType::U8x4)
            .into_diagnostic()?;

    let mut dst_image =
        fir::images::Image::new(target_width, target_height, src_image.pixel_type());

    let mut resizer = fir::Resizer::new();
    resizer
        .resize(
            &src_image,
            &mut dst_image,
            &ResizeOptions::new().resize_alg(fir::ResizeAlg::Nearest),
        )
        .into_diagnostic()?;

    // Luma subsampling to make the ratio 4:4:4 again
    let mut out = Vec::with_capacity(target_width as usize * target_height as usize * 3);
    dst_image
        .into_vec()
        .into_iter()
        .tuples::<(_, _, _, _)>()
        // PERF: We use extend here because calling map and then flattening is somehow *extremely* slow
        // Seems to be because of: https://github.com/rust-lang/rust/issues/79992#issuecomment-743937191
        .for_each(|(y1, u, y2, v)| {
            out.extend([((u16::from(y1) + u16::from(y2)) / 2) as u8, u, v]);
        });

    Ok(out)
}
