use crate::{
    behavior::engine::{Behavior, Context, Control},
    nao::manager::Priority,
};
use nidhogg::types::{color, FillExt, RightEye};

#[derive(Copy, Clone, Debug, Default)]
pub struct Example {
    iter: i32,
}

impl Behavior for Example {
    fn execute(&mut self, _context: Context, control: &mut Control) {
        self.iter += 1;

        let right_eye = if self.iter < 100 {
            RightEye::fill(color::f32::RED)
        } else {
            RightEye::fill(color::f32::BLUE)
        };

        control
            .nao_manager
            .set_right_eye_led(right_eye, Priority::Medium);
    }
}
