use crate::{
    behavior::engine::{Behavior, Context},
    motion::arbiter::MotionArbiter,
};
use nidhogg::{
    types::{color, FillExt, JointArray, RightEye},
    NaoControlMessage,
};

/// This is the default behavior of the robot.
/// In this state the robot does nothing and all motors are turned off.
/// In this state the robot has a blue right eye.
#[derive(Copy, Clone, Debug, Default)]
pub struct Passive;

impl Behavior for Passive {
    fn execute(
        &mut self,
        _context: Context,
        motion_arbiter: &mut MotionArbiter,
        control_msg: &mut NaoControlMessage,
    ) {
        // Makes right eye blue.
        control_msg.right_eye = RightEye::fill(color::f32::BLUE);
        // Turns off motors
        control_msg.stiffness = JointArray::fill(-1.0);
    }
}
