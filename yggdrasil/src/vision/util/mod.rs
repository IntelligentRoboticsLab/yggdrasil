pub mod bbox;

use bbox::{ConvertBbox, Xyxy};

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
