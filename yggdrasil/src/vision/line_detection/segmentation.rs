use nalgebra::DMatrix;

use super::YUVImage;

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

const HORIZONTAL_SPLITS: usize = 128;
const VERTICAL_SPLITS: usize = 160;

pub const HORIZONTAL_SPLIT_SIZE: usize = 1280 / HORIZONTAL_SPLITS;
pub const VERTICAL_SPLIT_SIZE: usize = 960 / VERTICAL_SPLITS;

pub type SegmentMatrix = DMatrix<Segment>;

pub fn segment_image(image: &YUVImage) -> SegmentMatrix {
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

    let mut segment_matrix = SegmentMatrix::from_element(
        VERTICAL_SPLITS,
        HORIZONTAL_SPLITS,
        Segment::new(0, 0, SegmentType::Other),
    );

    for i in 0..VERTICAL_SPLITS {
        for j in 0..HORIZONTAL_SPLITS {
            let segment = image.view(
                (i * VERTICAL_SPLIT_SIZE, j * HORIZONTAL_SPLIT_SIZE), 
                (VERTICAL_SPLIT_SIZE, HORIZONTAL_SPLIT_SIZE)
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

            let x = (j * HORIZONTAL_SPLIT_SIZE + HORIZONTAL_SPLIT_SIZE / 2) as u32;
            let y = (i * VERTICAL_SPLIT_SIZE + VERTICAL_SPLIT_SIZE / 2) as u32;
            segment_matrix[(i, j)] = Segment::new(x, y, seg_type);
        }
    }

    segment_matrix
}

pub fn draw_segments(segment_matrix: &SegmentMatrix, field_barrier: u32) {
    use image::{Rgb, RgbImage};

    let mut img = DMatrix::<(u8, u8, u8)>::from_element( 960, 1280, (255, 0, 0));

    segment_matrix.iter().for_each(|segment| {
            let color = match segment.seg_type {
                SegmentType::Other => (0, 0, 0),
                SegmentType::Field => (0, 255, 0),
                SegmentType::Line => (255, 255, 255)
            };

            img
            .view_mut( 
                ((segment.y as usize - VERTICAL_SPLIT_SIZE / 2), (segment.x as usize - HORIZONTAL_SPLIT_SIZE / 2)), 
                (VERTICAL_SPLIT_SIZE as usize, HORIZONTAL_SPLIT_SIZE as usize) )
            .fill(color);

    });

    img.row_mut(field_barrier as usize).fill((255, 0, 0));

    let img = RgbImage::from_fn(1280, 960, |x, y| {
        let (r, g, b) = img[(y as usize, x as usize)];
        Rgb([r, g, b])
    });

    img.save("line-detection_segmentation.png").unwrap();

}
