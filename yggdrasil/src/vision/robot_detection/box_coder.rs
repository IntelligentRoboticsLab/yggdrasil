use ndarray::{s, stack, Array2, Axis};
use num::complex::ComplexFloat;

/// Utility that decodes bounding boxes from the regression format output by the model.
///
/// Based on the implementation in [torchvision].
///
/// [torchvision]: https://github.com/pytorch/vision/blob/33db2b3ebfdd2f73a9228f430fa7dd91c3b18078/torchvision/models/detection/_utils.py#L129
pub struct BoxCoder {
    /// The weights used for decoding the bounding boxes.
    pub weights: (f32, f32, f32, f32),
    /// The maximum value for the bounding box transformation, before clamping.
    /// This is used to avoid overflow when applying the exponent.
    pub bbox_xform_clip: f32,
}

impl BoxCoder {
    /// Create a new [`BoxCoder`] with the given weights.
    ///
    /// The weights are used for the x, y, width, and height respectively
    /// of the bounding box.
    ///
    /// This will default to a `bbox_xform_clip` of `ln(1000/16)`.
    pub fn new(weights: (f32, f32, f32, f32)) -> Self {
        Self::new_with_clip(weights, (1000.0 / 16.0).ln())
    }

    /// Create a new [`BoxCoder`] with the given weights and clipping value.
    pub fn new_with_clip(weights: (f32, f32, f32, f32), bbox_xform_clip: f32) -> Self {
        BoxCoder {
            weights,
            bbox_xform_clip,
        }
    }

    /// Decode the relative bounding box predictions into xywh format.
    pub fn decode_single(&self, rel_codes: Array2<f32>, boxes: Array2<f32>) -> Array2<f32> {
        let num_features = boxes.dim().0;
        let widths = &boxes.column(2) - &boxes.column(0);
        let heights = &boxes.column(3) - &boxes.column(1);

        let center_x = &boxes.column(0) + (0.5 * &widths);
        let center_y = &boxes.column(1) + (0.5 * &heights);

        let (wx, wy, ww, wh) = self.weights;

        let dx = &rel_codes.slice(s![.., 0..;4]) / wx;
        let dy = &rel_codes.slice(s![.., 1..;4]) / wy;
        let dw = &rel_codes.slice(s![.., 2..;4]) / ww;
        let dh = &rel_codes.slice(s![.., 3..;4]) / wh;

        let dx = dx.to_shape(num_features).expect("Failed to reshape dx");
        let dy = dy.to_shape(num_features).expect("Failed to reshape dy");
        let dw = dw.to_shape(num_features).expect("Failed to reshape dw");
        let dh = dh.to_shape(num_features).expect("Failed to reshape dh");

        // clamp to avoid overflow in exp
        let dw = dw.mapv(|x| x.min(self.bbox_xform_clip));
        let dh = dh.mapv(|x| x.min(self.bbox_xform_clip));

        let pred_center_x = &dx * &widths + center_x;
        let pred_center_y = &dy * &heights + center_y;

        let pred_w = dw.mapv(|x| x.exp()) * widths;
        let pred_h = dh.mapv(|x| x.exp()) * heights;

        let center_to_center_height = pred_h / 2.0;
        let center_to_center_width = pred_w / 2.0;

        let pred_boxes1 = &pred_center_x - &center_to_center_width;
        let pred_boxes2 = &pred_center_y - &center_to_center_height;
        let pred_boxes3 = &pred_center_x + &center_to_center_width;
        let pred_boxes4 = &pred_center_y + &center_to_center_height;

        stack![Axis(1), pred_boxes1, pred_boxes2, pred_boxes3, pred_boxes4]
    }
}
