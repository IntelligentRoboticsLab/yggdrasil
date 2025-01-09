use crate::{
    behavior_old::engine::{Behavior, Context, Control},
    nao::Priority,
};
use nidhogg::types::{color, FillExt, RightEye};

// The robot shouldn't do anything while in unstiff state.
const UNSTIFF_PRIORITY: Priority = Priority::Critical;

/// This is often the starting behavior of the robot.
/// In this state the robot sits down, after which it unstiffens its legs, arms and head.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Sitting;

impl Behavior for Sitting {
    fn execute(&mut self, _context: Context, control: &mut Control) {
        // Makes right eye blue.
        control
            .nao_manager
            .set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::default());

        if control.walking_engine.is_sitting() {
            // Makes robot floppy except for hip joints, locked in sitting position.
            control.nao_manager.unstiff_sit(UNSTIFF_PRIORITY);
        } else {
            control.walking_engine.request_sit();
        }

        control.nao_manager.unstiff_arms(UNSTIFF_PRIORITY);
    }
}
