use ndarray::{s, stack, Array2, Axis};

pub struct BoxCoder {
    pub weights: (f32, f32, f32, f32),
    pub bbox_xform_clip: f32,
}

impl BoxCoder {
    /// Create a new [`BoxCoder`] with the given weights.
    ///
    /// This will default to a `bbox_xform_clip` of `ln(1000/16)`.
    pub fn new(weights: (f32, f32, f32, f32)) -> Self {
        BoxCoder {
            weights,
            bbox_xform_clip: (1000_f32 / 16_f32).ln(),
        }
    }

    /// Create a new [`BoxCoder`] with the given weights and clipping value.
    pub fn new_with_clip(weights: (f32, f32, f32, f32), bbox_xform_clip: f32) -> Self {
        BoxCoder {
            weights,
            bbox_xform_clip,
        }
    }

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

        let dx = dx.to_shape(num_features).unwrap();
        let dy = dy.to_shape(num_features).unwrap();
        let dw = dw.to_shape(num_features).unwrap();
        let dh = dh.to_shape(num_features).unwrap();

        // clamp to avoid overflow in exp
        let dw = dw.mapv(|x| x.min(self.bbox_xform_clip));
        let dh = dh.mapv(|x| x.min(self.bbox_xform_clip));

        let pred_center_x = &dx * &widths + center_x;
        let pred_center_y = &dy * &heights + center_y;

        let pred_w = dw.mapv(|x| x.exp()) * widths;
        let pred_h = dh.mapv(|x| x.exp()) * heights;

        let c_to_c_h = pred_h / 2.0;
        let c_to_c_w = pred_w / 2.0;

        let pred_boxes1 = &pred_center_x - &c_to_c_w;
        let pred_boxes2 = &pred_center_y - &c_to_c_h;
        let pred_boxes3 = &pred_center_x + &c_to_c_w;
        let pred_boxes4 = &pred_center_y + &c_to_c_h;

        stack![Axis(1), pred_boxes1, pred_boxes2, pred_boxes3, pred_boxes4]
    }
}
