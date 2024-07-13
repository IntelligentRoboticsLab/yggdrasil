use crate::{
    behavior::engine::{Behavior, Context, Control},
    nao::manager::Priority,
};
use nidhogg::types::{color, FillExt, RightEye};

// The robot shouldn't do anything while in unstiff state.
const UNSTIFF_PRIORITY: Priority = Priority::Critical;

/// This is often the starting behavior of the robot.
/// In this state the robot sits down, after which it unstiffens its legs, arms and head.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Unstiff;

impl Behavior for Unstiff {
    fn execute(&mut self, _context: Context, control: &mut Control) {
        // Makes right eye blue.
        control
            .nao_manager
            .set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::default());

        if !control.walking_engine.is_sitting() {
            control.walking_engine.request_sit();
        } else {
            control.nao_manager.unstiff_legs(UNSTIFF_PRIORITY);
        }

        control
            .nao_manager
            .unstiff_arms(UNSTIFF_PRIORITY)
            .unstiff_head(UNSTIFF_PRIORITY);
    }
}
