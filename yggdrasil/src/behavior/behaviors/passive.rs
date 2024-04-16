use crate::{
    behavior::engine::{Behavior, Context},
    motion::motion_manager::MotionManager,
    nao::manager::{NaoManager, Priority},
    walk::engine::WalkingEngine,
};
use nidhogg::types::{color, FillExt, RightEye};

/// This is the default behavior of the robot.
/// In this state the robot does nothing and all motors are turned off.
/// In this state the robot has a blue right eye.
#[derive(Copy, Clone, Debug, Default)]
pub struct Passive;

impl Behavior for Passive {
    fn execute(
        &mut self,
        _context: Context,
        nao_manager: &mut NaoManager,
        _walking_engine: &mut WalkingEngine,
        _: &mut MotionManager,
    ) {
        // Turns off motors
        nao_manager
            .unstiff_legs(Priority::Critical)
            .unstiff_arms(Priority::Critical)
            .unstiff_head(Priority::Critical);

        // Makes right eye blue.
        nao_manager.set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::default());
    }
}
