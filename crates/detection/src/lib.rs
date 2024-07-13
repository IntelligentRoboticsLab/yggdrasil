pub mod anchor;
pub mod box_coder;
pub mod meshgrid;
pub mod postprocess;

type BBox = (f32, f32, f32, f32);

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

fn union(box1: &BBox, box2: &BBox) -> f32 {
    let area1 = (box1.2 - box1.0) * (box1.3 - box1.1);
    let area2 = (box2.2 - box2.0) * (box2.3 - box2.1);
    area1 + area2 - intersection(box1, box2)
}

pub fn iou(box1: &BBox, box2: &BBox) -> f32 {
    let intersect = intersection(box1, box2);
    let union = union(box1, box2);

    intersect / union
}
