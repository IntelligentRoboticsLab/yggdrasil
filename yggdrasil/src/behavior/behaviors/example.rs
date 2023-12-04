use crate::behavior::engine::{Context, Execute, Role};
use nidhogg::{
    types::{Color, FillExt, RightEye},
    NaoControlMessage,
};

#[derive(Copy, Clone, Debug, Default)]
pub struct Example {
    iter: i32,
}

impl Execute for Example {
    fn execute(
        &mut self,
        _context: Context,
        _current_role: &Role,
        control_message: &mut NaoControlMessage,
    ) {
        self.iter += 1;

        println!("{}", self.iter);

        let right_eye = if self.iter < 100 {
            RightEye::fill(Color::RED)
        } else {
            RightEye::fill(Color::BLUE)
        };

        _control_message.right_eye = right_eye;
    }
}
