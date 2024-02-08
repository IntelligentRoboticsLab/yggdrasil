use heimdall::YuyvImage;
use image::codecs::jpeg::JpegEncoder;
use nalgebra::DMatrix;
use std::fs::File;

use miette::{IntoDiagnostic, Result};

use image::imageops::FilterType;
use plotters::prelude::*;

use super::{
    ransac::fit_lines,
    segmentation::{draw_segments, segment_image, Segment, SegmentType},
    Line, LineDetectionConfig, YUVImage,
};

pub fn detect_lines(config: LineDetectionConfig, image: &YuyvImage) -> Vec<Line> {
    let yuv_tuples: Vec<_> = image.yuv_row_iter().rev().collect();

    let yuv_image = YUVImage::from_row_slice(image.height(), image.width(), &yuv_tuples);

    let segmented_image = segment_image(&config, &yuv_image);

    let mut field_barrier_segment = 0;
    for i in 0..segmented_image.nrows() {
        let row = segmented_image.row(i);

        let field_segments = row
            .iter()
            .filter(|x| x.seg_type == SegmentType::Field)
            .count();

        if field_segments > row.len() / (config.field_barrier_percentage * 10.0) as usize {
            break;
        }

        field_barrier_segment = i as u32;
    }

    let field_barrier =
        field_barrier_segment * image.height() as u32 / config.vertical_splits as u32;

    draw_segments(&config, &yuv_image, &segmented_image, field_barrier);

    let lines = fit_lines(
        &config,
        &segmented_image
            .iter()
            .filter(|x| x.seg_type == SegmentType::Line && x.y < field_barrier)
            .copied()
            .collect::<Vec<Segment>>(),
    );

    lines
}

// Code usefull for debugging intermediate images
#[allow(dead_code)]
fn save_sub_image_jpeg(image: &DMatrix<(u8, u8, u8)>, filename: &str) -> Result<()> {
    let image_rgb = image
        .iter()
        .flat_map(|x| {
            fn clamp(value: i32) -> u8 {
                #[allow(clippy::cast_sign_loss)]
                #[allow(clippy::cast_possible_truncation)]
                return value.clamp(0, 255) as u8;
            }

            let (y, u, v) = x;

            let y = i32::from(*y) - 16;
            let u = i32::from(*u) - 128;
            let v = i32::from(*v) - 128;

            let r = (298 * y + 409 * v + 128) >> 8;
            let g = (298 * y - 100 * u - 208 * v + 128) >> 8;
            let b = (298 * y + 516 * u + 128) >> 8;

            vec![clamp(r), clamp(g), clamp(b)]
        })
        .collect::<Vec<u8>>();

    let output_file = File::create(filename).into_diagnostic()?;
    let mut encoder = JpegEncoder::new(output_file);

    encoder
        .encode(
            &image_rgb,
            image.ncols() as u32,
            image.nrows() as u32,
            image::ColorType::Rgb8,
        )
        .into_diagnostic()?;

    Ok(())
}

const OUT_FILE_NAME: &str = "line-detection_result.png";

pub fn plot_image(lines: Vec<Line>, image: &YuyvImage) -> Result<()> {
    // Convert the image to something image::load can read

    let image_buffer = image::ImageBuffer::from_raw(1280, 960, image.to_rgb()?.to_vec()).unwrap();

    // Mirror the y axis
    let mut lines = lines;
    for line in &mut lines {
        line.y1 = 960 - line.y1;
        line.y2 = 960 - line.y2;
    }

    let root = BitMapBackend::new(OUT_FILE_NAME, (1280, 960)).into_drawing_area();
    root.fill(&WHITE).into_diagnostic()?;

    let mut chart = ChartBuilder::on(&root)
        .build_cartesian_2d(0.0..1280., 0.0..960.)
        .into_diagnostic()?;

    chart
        .configure_mesh()
        .disable_mesh()
        .draw()
        .into_diagnostic()?;

    let (w, h) = chart.plotting_area().dim_in_pixel();

    let image = image::DynamicImage::ImageRgb8(image_buffer)
        .resize_exact(w, h, FilterType::Nearest)
        .flipv()
        .fliph();

    let elem: BitMapElement<_> = ((0., 960.), image).into();

    chart.draw_series(std::iter::once(elem)).into_diagnostic()?;

    for line in lines {
        chart
            .draw_series(LineSeries::new(
                vec![
                    (line.x1 as f64, line.y1 as f64),
                    (line.x2 as f64, line.y2 as f64),
                ],
                Into::<ShapeStyle>::into(RED).stroke_width(5),
            ))
            .into_diagnostic()?;
    }

    // To avoid the IO failure being ignored silently, we manually call the present function
    root.present().expect("Unable to write result to file");
    // println!("Result has been saved to {}", OUT_FILE_NAME);
    Ok(())
}
