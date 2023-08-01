#![feature(portable_simd)]
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use std::simd::Simd;

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n - 1) + fibonacci(n - 2),
    }
}

fn clip(value: i32) -> u8 {
    i32::max(0, i32::min(255, value)) as u8
}

fn yuyv444_to_rgb_parallel(y: u8, u: u8, v: u8) -> (u8, u8, u8) {
    let c = y as i32 - 16;
    let d = u as i32 - 128;
    let e = v as i32 - 128;

    let red = (298 * c + 409 * e + 128) >> 8;
    let green = (298 * c - 100 * d - 208 * e + 128) >> 8;
    let blue = (298 * c + 516 * d + 128) >> 8;

    (clip(red), clip(green), clip(blue))
}

fn yuyv444_to_rgb_simd(y: u8, u: u8, v: u8) -> (u8, u8, u8) {
    let yyy = Simd::from([y as i32, y as i32, y as i32, 0]) - Simd::from([16, 16, 16, 0]);
    let uuu = Simd::from([u as i32, u as i32, u as i32, 0]) - Simd::from([128, 128, 128, 0]);
    let vvv = Simd::from([v as i32, v as i32, v as i32, 0]) - Simd::from([128, 128, 128, 0]);

    let rgb = (yyy * Simd::from([298, 298, 298, 0])
        + uuu * Simd::from([0, -100, 516, 0])
        + vvv * Simd::from([409, -208, 0, 0])
        + Simd::from([128, 128, 128, 0]))
        >> Simd::from([8, 8, 8, 0]);

    (clip(rgb[0]), clip(rgb[1]), clip(rgb[2]))
}

fn yuyv422_to_rgb_simd(y1: u8, u: u8, y2: u8, v: u8) -> ((u8, u8, u8), (u8, u8, u8)) {
    fn clip(value: i32) -> u8 {
        i32::max(0, i32::min(255, value)) as u8
    }

    let yyy1_yyy2 = Simd::from([
        y1 as i32, y1 as i32, y1 as i32, y2 as i32, y2 as i32, y2 as i32, 0, 0,
    ]) - Simd::from([16, 16, 16, 16, 16, 16, 0, 0]);
    let uuu_uuu = Simd::from([
        u as i32, u as i32, u as i32, u as i32, u as i32, u as i32, 0, 0,
    ]) - Simd::from([128, 128, 128, 128, 128, 128, 0, 0]);
    let vvv = Simd::from([
        v as i32, v as i32, v as i32, v as i32, v as i32, v as i32, 0, 0,
    ]) - Simd::from([128, 128, 128, 128, 128, 128, 0, 0]);

    let rgb1_rgb2 = (yyy1_yyy2 * Simd::from([298, 298, 298, 298, 298, 298, 0, 0])
        + uuu_uuu * Simd::from([0, -100, 516, 0, -100, 516, 0, 0])
        + vvv * Simd::from([409, -208, 0, 409, -208, 0, 0, 0])
        + Simd::from([128, 128, 128, 128, 128, 128, 0, 0]))
        >> Simd::from([8, 8, 8, 8, 8, 8, 0, 0]);

    (
        (clip(rgb1_rgb2[0]), clip(rgb1_rgb2[1]), clip(rgb1_rgb2[2])),
        (clip(rgb1_rgb2[3]), clip(rgb1_rgb2[4]), clip(rgb1_rgb2[5])),
    )
}

fn yuyv_benchmark_parallel(c: &mut Criterion) {
    let y1 = 20;
    let u = 30;
    let y2 = 40;
    let v = 50;

    c.bench_function("yuyv benchmark parallel", |b| {
        b.iter(|| {
            yuyv444_to_rgb_parallel(black_box(y1), black_box(u), black_box(v));
            yuyv444_to_rgb_parallel(black_box(y2), black_box(u), black_box(v))
        })
    });
}

fn yuyv_benchmark_simd(c: &mut Criterion) {
    let y1 = 20;
    let u = 30;
    let y2 = 40;
    let v = 50;

    c.bench_function("yuyv benchmark simd", |b| {
        b.iter(|| {
            yuyv444_to_rgb_simd(black_box(y1), black_box(u), black_box(v));
            yuyv444_to_rgb_simd(black_box(y2), black_box(u), black_box(v))
        })
    });
}

fn yuyv_benchmark_simd_improved(c: &mut Criterion) {
    let y1 = 20;
    let u = 30;
    let y2 = 40;
    let v = 50;

    c.bench_function("yuyv benchmark simd improved", |b| {
        b.iter(|| {
            yuyv422_to_rgb_simd(black_box(y1), black_box(u), black_box(y2), black_box(v));
        })
    });
}

criterion_group!(
    benches,
    yuyv_benchmark_parallel,
    yuyv_benchmark_simd,
    yuyv_benchmark_simd_improved
);
criterion_main!(benches);
