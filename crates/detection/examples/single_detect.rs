use ndarray::Array3;

fn main() {
    let anchor_generator =
        detection::anchor::DefaultBoxGenerator::new(vec![vec![0.4, 0.5], vec![0.85]], 0.15, 0.9);

    let boxes = anchor_generator.create_boxes((100, 100), Array3::zeros((32, 12, 12)));

    println!("{:?}", boxes);
}
