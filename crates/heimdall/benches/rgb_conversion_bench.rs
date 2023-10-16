use std::io::Write;

use heimdall::{Camera, RgbImage, CAMERA_TOP};

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let mut camera = Camera::new(CAMERA_TOP).unwrap();
    let image = camera.get_yuyv_image().unwrap();

    let mut file = std::fs::File::create("image.raw").unwrap();
    file.write_all(&image).unwrap();

    let mut rgb_image = RgbImage::new();

    c.bench_function("rgb conversion", |b| {
        b.iter(|| image.to_rgb(black_box(&mut rgb_image)));
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
