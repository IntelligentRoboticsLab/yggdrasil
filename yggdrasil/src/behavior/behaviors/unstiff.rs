use crate::{
    behavior::engine::{Behavior, Context},
    nao::manager::{NaoManager, Priority},
    walk::engine::WalkingEngine,
};
use nidhogg::types::{color, FillExt, RightEye};

/// This is the default behavior of the robot.
/// In this state the robot does nothing and retains its previous position.
/// In this state the robot has a blue right eye.
#[derive(Copy, Clone, Debug, Default)]
pub struct Unstiff;

impl Behavior for Unstiff {
    fn execute(
        &mut self,
        _context: Context,
        nao_manager: &mut NaoManager,
        walking_engine: &mut WalkingEngine,
    ) {
        // TODO: sit down
        println!("We unstiffin");

        nao_manager
            .unstiff_legs(Priority::Critical)
            .unstiff_arms(Priority::Critical)
            .unstiff_head(Priority::Critical);

        // Makes right eye blue.
        nao_manager.set_right_eye_led(RightEye::fill(color::f32::BLUE), Priority::default());
    }
}
