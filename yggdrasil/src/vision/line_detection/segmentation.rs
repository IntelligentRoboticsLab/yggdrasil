use nalgebra::DMatrix;

use super::{YUVImage, LineDetectionConfig};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SegmentType {
    Other,
    Field,
    Line,
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Segment {
    pub x: u32,
    pub y: u32,
    pub seg_type: SegmentType,
}

impl Segment {
    pub fn new(x: u32, y: u32, seg_type: SegmentType) -> Self {
        Segment { x, y, seg_type }
    }
}

pub type SegmentMatrix = DMatrix<Segment>;

pub fn segment_image(config: &LineDetectionConfig, image: &YUVImage) -> SegmentMatrix {
    fn get_segment_type(pixel: (u8, u8, u8)) -> SegmentType {
        let (y, u, v) = pixel;
        if (y > 65) && (y < 140) && (u > 90) && (u < 110) && (v > 115) && (v < 135) {
            SegmentType::Field
        } else if (y > 190) && (u > 110) && (u < 140) && (v > 115) && (v < 140) {
            SegmentType::Line
        } else {
            SegmentType::Other
        }
    }

    let vertical_splits = config.vertical_splits;
    let horizontal_splits = config.horizontal_splits;

    let vertical_split_size = image.nrows() / vertical_splits;
    let horizontal_split_size = image.ncols() / horizontal_splits;

    let mut segment_matrix = SegmentMatrix::from_element(
        vertical_splits,
        horizontal_splits,
        Segment::new(0, 0, SegmentType::Other),
    );

    (0..vertical_splits).for_each(|row| {
        (0..horizontal_splits).for_each(|column| {
            let segment = image.view(
                (row * vertical_split_size, column * horizontal_split_size), 
                (vertical_split_size, horizontal_split_size)
            );     

            let mut field_sum: u32 = 0;
            let mut line_sum: u32 = 0;
            let mut other_sum: u32 = 0;

            for pixel in segment.iter() {
                match get_segment_type(*pixel) {
                    SegmentType::Field => field_sum += 1,
                    SegmentType::Line => line_sum += 1,
                    SegmentType::Other => other_sum += 1,
                }
            }

            let seg_type = if line_sum > other_sum {
                SegmentType::Line
            } else if field_sum > line_sum && field_sum > other_sum {
                SegmentType::Field
            } else {
                SegmentType::Other
            };

            let x = (column * horizontal_split_size + horizontal_split_size / 2) as u32;
            let y = (row * vertical_split_size + vertical_split_size / 2) as u32;
            segment_matrix[(row, column)] = Segment::new(x, y, seg_type);
        });
    });

    segment_matrix
}

pub fn draw_segments(config: &LineDetectionConfig,image: &YUVImage, segment_matrix: &SegmentMatrix, field_barrier: u32) {
    use image::{Rgb, RgbImage};

    let vertical_split_size = image.nrows() / config.vertical_splits;
    let horizontal_split_size = image.ncols() / config.horizontal_splits;

    let mut img = DMatrix::<(u8, u8, u8)>::from_element(image.nrows(), image.ncols(), (255, 0, 0));

    segment_matrix.iter().for_each(|segment| {
            let color = match segment.seg_type {
                SegmentType::Other => (0, 0, 0),
                SegmentType::Field => (0, 255, 0),
                SegmentType::Line => (255, 255, 255)
            };

            img
            .view_mut( 
                ((segment.y as usize - vertical_split_size / 2), (segment.x as usize - horizontal_split_size / 2)), 
                (vertical_split_size, horizontal_split_size) )
            .fill(color);

    });

    img.row_mut(field_barrier as usize).fill((255, 0, 0));

    let img = RgbImage::from_fn(image.ncols() as u32, image.nrows() as u32, |x, y| {
        let (r, g, b) = img[(y as usize, x as usize)];
        Rgb([r, g, b])
    });

    img.save("line-detection_segmentation.png").unwrap();

}
