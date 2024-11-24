use std::time::Duration;

use crate::{
    behavior::engine::{Behavior, Context, Control},
    nao::Priority,
};
use nidhogg::types::HeadJoints;

const HEAD_STIFFNESS: f32 = 0.2;
const HEAD_ROTATION_TIME: Duration = Duration::from_millis(500);

/// Stand up and stop walking, while looking straight ahead.
/// This is used for when the robot is penalized and not allowed to perform any actions.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Stand;

impl Behavior for Stand {
    fn execute(&mut self, _context: Context, control: &mut Control) {
        control.walking_engine.request_stand();
        control.walking_engine.end_step_phase();

        control.nao_manager.set_head_target(
            HeadJoints::default(),
            HEAD_ROTATION_TIME,
            Priority::default(),
            HEAD_STIFFNESS,
        );
    }
}
