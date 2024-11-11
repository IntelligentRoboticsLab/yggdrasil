use crate::{
    behavior::engine::{Behavior, Context, Control},
    nao::{Priority, HeadTarget},
};
use nalgebra::Point3;
use nidhogg::types::{color, FillExt, HeadJoints, RightEye};

// The robot shouldn't do anything while in unstiff state.
const UNSTIFF_PRIORITY: Priority = Priority::Critical;

const HEAD_STIFFNESS: f32 = 0.4;

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

        let test_point3 = Point3::new(10.0, 0.0, 0.5);
        let look_at_test = _context.pose.get_look_at_absolute(&test_point3);
        
        if control.walking_engine.is_sitting() {
            // Makes robot floppy except for hip joints, locked in sitting position.
            control.nao_manager.unstiff_sit(UNSTIFF_PRIORITY);
        } else {
            control.walking_engine.request_sit();
        }

        if let HeadTarget::None = control.nao_manager.head_target {
            control.nao_manager.set_head_target(
                look_at_test,
            );
        }

        control
            .nao_manager
            .unstiff_arms(UNSTIFF_PRIORITY);
            // .unstiff_head(UNSTIFF_PRIORITY);
    }
}
