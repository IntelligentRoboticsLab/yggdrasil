use ndarray::{concatenate, s, stack, Array, Array2, Array3, ArrayView, Axis};

use crate::meshgrid::{meshgrid, Indexing};
use itertools::{repeat_n, Itertools};

#[derive(Debug, Clone)]
pub struct DefaultBoxGenerator {
    pub aspect_ratios: Vec<Vec<f32>>,
    pub scales: Vec<f32>,
    pub wh_pairs: Vec<Array2<f32>>,
}

impl DefaultBoxGenerator {
    pub fn new(
        aspect_ratios: Vec<Vec<f32>>,
        min_ratio: f32,
        max_ratio: f32,
    ) -> DefaultBoxGenerator {
        let num_outputs = aspect_ratios.len();
        let scales = Self::create_scales(num_outputs, min_ratio, max_ratio);
        let wh_pairs = Self::create_width_height_pairs(num_outputs, &aspect_ratios, &scales);

        DefaultBoxGenerator {
            aspect_ratios,
            scales,
            wh_pairs,
        }
    }

    /// Create a list of scales based on the number of outputs and the min and max ratios.
    /// The scales are evenly distributed between the min and max ratios, and the
    /// last scale is always `1.0`.
    ///
    /// Default values for `min_ratio` and `max_ratio` are `0.2` and `0.9`, respectively.
    fn create_scales(num_outputs: usize, min_ratio: f32, max_ratio: f32) -> Vec<f32> {
        let ratio = max_ratio - min_ratio;
        let mut scales = Vec::with_capacity(num_outputs + 1);
        for i in 0..num_outputs {
            scales.push(min_ratio + ratio * i as f32 / (num_outputs - 1) as f32);
        }

        scales.push(1.0);

        scales
    }

    /// Generate width and height pairs for each feature scale.
    ///
    /// The pairs are generated based on the aspect ratios and scales, following the SSD paper.
    /// For each scale, the following pairs are generated:
    /// - the scale itself
    /// - the square root of the scale and the next scale
    /// - the scale times the square root of the aspect ratio
    /// - the scale divided by the square root of the aspect ratio
    fn create_width_height_pairs(
        num_outputs: usize,
        aspect_ratios: &Vec<Vec<f32>>,
        scales: &[f32],
    ) -> Vec<Array2<f32>> {
        let mut pairs = Vec::with_capacity(num_outputs);
        for i in 0..num_outputs {
            let scale = scales[i];
            let next_scale = scales[i + 1];

            let scale_prime = (scale * next_scale).sqrt();
            let mut scale_pairs =
                Array2::from_shape_vec((2, 2), vec![scale, scale, scale_prime, scale_prime])
                    .unwrap();

            aspect_ratios[i].iter().for_each(|a| {
                let sqrt_ar = a.sqrt();
                let width = scale * sqrt_ar;
                let height = scale / sqrt_ar;

                scale_pairs
                    .push_row(ArrayView::from(&[width, height]))
                    .unwrap();
                scale_pairs
                    .push_row(ArrayView::from(&[height, width]))
                    .unwrap();
            });

            pairs.push(scale_pairs);
        }

        pairs
    }

    pub fn create_boxes(&self, image_size: (usize, usize), features: Array3<f32>) -> Array2<f32> {
        // create default boxes for each feature map, in cx, cy, w, h format
        let mut default_boxes = self.grid_default_boxes((12, 12));

        let (image_width, image_height) = (image_size.1 as f32, image_size.0 as f32);

        // convert the default boxes to xyxy format
        for i in 0..default_boxes.dim().0 {
            let (cx, cy, w, h) = (
                default_boxes[[i, 0]],
                default_boxes[[i, 1]],
                default_boxes[[i, 2]],
                default_boxes[[i, 3]],
            );

            default_boxes[[i, 0]] = (cx - w / 2.0) * image_width;
            default_boxes[[i, 1]] = (cy - h / 2.0) * image_height;
            default_boxes[[i, 2]] = (cx + w / 2.0) * image_width;
            default_boxes[[i, 3]] = (cy + h / 2.0) * image_height;
        }

        default_boxes
    }

    /// Generate default boxes for a grid of feature maps.
    fn grid_default_boxes(&self, feature_size: (usize, usize)) -> Array2<f32> {
        let (y_fk, x_fk) = feature_size;

        let shifts_x = (Array::range(0.0, x_fk as f32, 1.0) + 0.5) / x_fk as f32;
        let shifts_y = (Array::range(0.0, y_fk as f32, 1.0) + 0.5) / y_fk as f32;

        let grids = meshgrid(&[shifts_y, shifts_x], Indexing::Ij).unwrap();

        let shift_y = grids[0].clone().into_shape((144, 1)).unwrap();
        let shift_x = grids[1].clone().into_shape((144, 1)).unwrap();

        let num_pairs = self.wh_pairs[0].dim().0;

        // repeat the shifts for each pair of width and height
        let shift = concatenate![Axis(1), shift_y, shift_x];
        let shifts = repeat_n(shift, num_pairs)
            .reduce(|acc, x| concatenate!(Axis(0), acc, x))
            .unwrap();

        // clip the default boxes, while they're encoded in cxcywh format
        let wh_pair = self.wh_pairs[0].map(|x| x.clamp(0.0, 1.0));
        let wh_pairs = repeat_n(wh_pair, y_fk * x_fk)
            .reduce(|acc, x| concatenate!(Axis(0), acc, x))
            .unwrap();

        concatenate![Axis(1), shifts, wh_pairs]
    }
}
