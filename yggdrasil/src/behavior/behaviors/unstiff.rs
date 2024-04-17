use crate::{
    behavior::engine::{Behavior, Context},
    motion::motion_manager::MotionManager,
    nao::manager::{NaoManager, Priority},
    walk::engine::WalkingEngine,
};
use nidhogg::types::{color, FillExt, RightEye};

/// This is often the starting behavior of the robot.
/// In this state the robot sits down, after which it unstiffens its legs, arms and head.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Unstiff;

impl Behavior for Unstiff {
    fn execute(
        &mut self,
        _context: Context,
        nao_manager: &mut NaoManager,
        walking_engine: &mut WalkingEngine,
        _: &mut MotionManager,
    ) {
        // Makes right eye blue.
        nao_manager.set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::default());

        if !walking_engine.is_sitting() {
            walking_engine.request_sit();
        } else {
            nao_manager.unstiff_legs(Priority::Critical);
        }

        nao_manager
            .unstiff_arms(Priority::Critical)
            .unstiff_head(Priority::Critical);
    }
}
