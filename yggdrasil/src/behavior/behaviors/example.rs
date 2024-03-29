use crate::{
    behavior::engine::{Behavior, Context},
    nao::manager::{NaoManager, Priority},
};
use nidhogg::types::{color, FillExt, RightEye};

#[derive(Copy, Clone, Debug, Default)]
pub struct Example {
    iter: i32,
}

impl Behavior for Example {
    fn execute(&mut self, _context: Context, nao_manager: &mut NaoManager) {
        self.iter += 1;

        let right_eye = if self.iter < 100 {
            RightEye::fill(color::f32::RED)
        } else {
            RightEye::fill(color::f32::BLUE)
        };

        nao_manager.set_right_eye_led(right_eye, Priority::Medium);
    }
}
