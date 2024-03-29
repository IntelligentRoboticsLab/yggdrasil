use crate::{
    behavior::engine::{Behavior, Context},
    nao::manager::{NaoManager, Priority},
};
use nidhogg::types::{color, FillExt, RightEye};

/// This is the default behavior of the robot.
/// In this state the robot does nothing and all motors are turned off.
/// In this state the robot has a blue right eye.
#[derive(Copy, Clone, Debug, Default)]
pub struct Passive;

impl Behavior for Passive {
    fn execute(&mut self, _context: Context, nao_manager: &mut NaoManager) {
        // Turns off motors
        nao_manager
            .unstiff_legs(Priority::default())
            .unstiff_arms(Priority::default())
            .unstiff_head(Priority::default());

        // Makes right eye blue.
        nao_manager.set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::default());
    }
}
