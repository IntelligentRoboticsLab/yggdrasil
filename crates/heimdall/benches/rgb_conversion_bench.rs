use std::io::Write;

use heimdall::{Camera, CAMERA_TOP};

use criterion::{criterion_group, criterion_main, Criterion};

fn criterion_benchmark(c: &mut Criterion) {
    let mut camera = Camera::new(CAMERA_TOP).unwrap();
    let image = camera.get_yuyv_image().unwrap();

    let mut file = std::fs::File::create("image.raw").unwrap();
    file.write_all(&image).unwrap();

    c.bench_function("rgb conversion", |b| {
        b.iter(|| image.to_rgb());
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
