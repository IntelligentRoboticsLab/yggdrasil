use ndarray::{concatenate, s, stack, Array2, ArrayView2, Axis};

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
        let widths = &boxes.column(2) - &boxes.column(0);
        let heights = &boxes.column(3) - &boxes.column(1);

        let ctr_x = &boxes.column(0) + &widths / 2.0;
        let ctr_y = &boxes.column(1) + &heights / 2.0;

        let (wx, wy, ww, wh) = self.weights;

        let dx = &rel_codes.column(0) / wx;
        let dy = &rel_codes.column(1) / wy;
        let dw = &rel_codes.column(2) / ww;
        let dh = &rel_codes.column(3) / wh;

        // clamp to avoid overflow in exp
        let dw = dw.mapv(|x| x.min(self.bbox_xform_clip));
        let dh = dh.mapv(|x| x.min(self.bbox_xform_clip));

        let pred_ctr_x = dx * &widths + ctr_x;
        let pred_ctr_y = dy * &heights + ctr_y;

        let pred_w = dw.mapv(|x| x.exp()) * widths;
        let pred_h = dh.mapv(|x| x.exp()) * heights;

        let c_to_c_h = pred_h / 2.0;
        let c_to_c_w = pred_w / 2.0;

        let x1 = &pred_ctr_x - &c_to_c_w;
        let y1 = &pred_ctr_y - &c_to_c_h;
        let x2 = &pred_ctr_x + &c_to_c_w;
        let y2 = &pred_ctr_y + &c_to_c_h;

        stack![Axis(1), x1, y1, x2, y2]
    }
}
