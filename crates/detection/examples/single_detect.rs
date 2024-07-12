use detection::box_coder;
use ndarray::{Array2, Array3};

fn main() {
    let anchor_generator =
        detection::anchor::DefaultBoxGenerator::new(vec![vec![0.4, 0.5], vec![0.85]], 0.15, 0.9);

    let boxes = anchor_generator.create_boxes((100, 100), Array3::zeros((64, 12, 12)));

    let box_coder = box_coder::BoxCoder::new((10.0, 10.0, 5.0, 5.0));

    let boxes = box_coder.decode_single(Array2::zeros((864, 4)), boxes);

    println!("{:?}", boxes);
}
