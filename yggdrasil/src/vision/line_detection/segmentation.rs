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

pub type SegmentMatrix = DMatrix<Segment>;

pub fn segment_image(image: &YUVImage) -> SegmentMatrix {
    let horizontal_splits = 128;
    let vertical_splits = 160;

    let horizontal_split_size = 960 / horizontal_splits;
    let vertical_split_size = 1280 / vertical_splits;

    fn is_green(pixel: (u8, u8, u8)) -> bool {
        let (y, u, v) = pixel;
        (y > 65) && (y < 140) && (u > 90) && (u < 110) && (v > 115) && (v < 135)
    }

    fn is_white(pixel: (u8, u8, u8)) -> bool {
        let (y, u, v) = pixel;
        (y > 190) && (u > 110) && (u < 140) && (v > 115) && (v < 140)
    }

    let mut segment_matrix = SegmentMatrix::from_element(
        horizontal_splits,
        vertical_splits,
        Segment::new(0, 0, SegmentType::Other),
    );

    for j in 0..vertical_splits {
        for i in 0..horizontal_splits {
            let segment = image.view(
                (i * horizontal_split_size, j * vertical_split_size),
                (horizontal_split_size, vertical_split_size),
            );

            let mut red_sum: u32 = 0;
            let mut green_sum: u32 = 0;
            let mut blue_sum: u32 = 0;

            for pixel in segment.iter() {
                red_sum += pixel.0 as u32;
                green_sum += pixel.1 as u32;
                blue_sum += pixel.2 as u32;
            }

            let red_average = red_sum / (segment.nrows() * segment.ncols()) as u32;
            let green_average = green_sum / (segment.nrows() * segment.ncols()) as u32;
            let blue_average = blue_sum / (segment.nrows() * segment.ncols()) as u32;
            let average = (red_average as u8, green_average as u8, blue_average as u8);
            if is_white(average) {
                segment_matrix[(i, j)] = Segment::new(
                    (j * vertical_split_size + vertical_split_size / 2) as u32,
                    (i * horizontal_split_size + horizontal_split_size / 2) as u32,
                    SegmentType::Line,
                );
            } else if is_green(average) {
                segment_matrix[(i, j)] = Segment::new(
                    (j * vertical_split_size + vertical_split_size / 2) as u32,
                    (i * horizontal_split_size + horizontal_split_size / 2) as u32,
                    SegmentType::Field,
                );
            }
        }
    }

    segment_matrix
}
