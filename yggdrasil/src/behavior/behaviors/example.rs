use crate::behavior::engine::{Behavior, Context};
use nidhogg::{
    types::{Color, FillExt, RightEye},
    NaoControlMessage,
};

#[derive(Copy, Clone, Debug, Default)]
pub struct Example {
    iter: i32,
}

impl Behavior for Example {
    fn execute(&mut self, _context: Context, control_message: &mut NaoControlMessage) {
        self.iter += 1;

        let right_eye = if self.iter < 100 {
            RightEye::fill(Color::RED)
        } else {
            RightEye::fill(Color::BLUE)
        };

        control_message.right_eye = right_eye;
    }
}
