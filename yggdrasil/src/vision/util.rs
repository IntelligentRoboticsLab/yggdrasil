pub fn non_max_suppression(boxes: Vec<BBox>, scores: Vec<f32>, threshold: f32) -> Vec<usize> {
    let mut final_indices = Vec::new();

    for i in 0..boxes.len() {
        let mut discard = false;
        for j in 0..boxes.len() {
            if i == j {
                continue;
            }

            let overlap = iou(&boxes[i], &boxes[j]);
            let score_i = scores[i];
            let score_j = scores[j];

            if overlap >= threshold && score_j >= score_i {
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

pub type BBox = (f32, f32, f32, f32);

pub fn intersection(box1: &BBox, box2: &BBox) -> f32 {
    let x1 = box1.0.max(box2.0);
    let y1 = box1.1.max(box2.1);
    let x2 = box1.2.min(box2.2);
    let y2 = box1.3.min(box2.3);

    if x2 < x1 || y2 < y1 {
        0.0
    } else {
        (x2 - x1) * (y2 - y1)
    }
}

pub fn union(box1: &BBox, box2: &BBox) -> f32 {
    let area1 = (box1.2 - box1.0) * (box1.3 - box1.1);
    let area2 = (box2.2 - box2.0) * (box2.3 - box2.1);
    area1 + area2 - intersection(box1, box2)
}

pub fn iou(box1: &BBox, box2: &BBox) -> f32 {
    let intersect = intersection(box1, box2);
    let union = union(box1, box2);

    intersect / union
}
