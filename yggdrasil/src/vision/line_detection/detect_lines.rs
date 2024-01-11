use heimdall::YuyvImage;
use image::codecs::jpeg::JpegEncoder;
use nalgebra::DMatrix;
use std::fs::File;
use std::time::Instant;

use miette::{IntoDiagnostic, Result};

use image::imageops::FilterType;
use plotters::prelude::*;

use super::{YUVImage, LineDetectionConfig};

use super::ransac::fit_lines;
use super::segmentation::{Segment, SegmentType, draw_segments};
use super::Line;

use super::segmentation::segment_image;

pub fn detect_lines(config: LineDetectionConfig, image: &YuyvImage) -> Vec<Line> {
    let before = Instant::now();
    
    let yuv_tuples = image
    .chunks_exact(4)
    .flat_map(|x| IntoIterator::into_iter([(x[0], x[1], x[3]), (x[2], x[1], x[3])]));

    let yuv_image = YUVImage::from_iterator(image.width() as usize, image.height() as usize, yuv_tuples).transpose();

    let segmented_image = segment_image(&config, &yuv_image);

    // Simple field boundary detection, Time 0ms
    let field_barrier_segment = segmented_image.row_iter().enumerate().find(|(_, row)| {
        let field_segments = row.iter().filter(|x| x.seg_type == SegmentType::Field).count();
        
        field_segments > row.len() / (config.field_barrier_percentage * 10.0) as usize
    }).map(|(i, _)| i).unwrap_or(0) as u32;
    
    let field_barrier = field_barrier_segment * image.height() / config.vertical_splits as u32;
    draw_segments(&config, &yuv_image, &segmented_image, field_barrier);
    
    let lines = fit_lines(
        &config,
        &segmented_image
        .iter()
        .filter(|x| x.seg_type == SegmentType::Line && x.y > field_barrier
        ).map(|x| *x)
        .collect::<Vec<Segment>>());
    
    println!("Elapsed time: {:.2?}", before.elapsed());
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
