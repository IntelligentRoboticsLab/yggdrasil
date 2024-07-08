use crate::{
    behavior::engine::{Behavior, Context, Control},
    nao::manager::Priority,
};
use nalgebra::Point2;
use nidhogg::types::{FillExt, HeadJoints};

const ROTATION_STIFFNESS: f32 = 0.3;

/// During a match the chest button is pressed before starting a match.
/// Once this is done, the robots are placed at the edge of the field from
/// which they will walk to their `Ready` positions.
///
/// This is the behaviour of the robot once the chest button is pressed.
/// In this state the robot will stand up straight and look at the middle
/// circle to make it easier to place the robot in the correct position.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct StandingLookAt {
    pub target: Point2<f32>,
}

impl Behavior for StandingLookAt {
    fn execute(&mut self, context: Context, control: &mut Control) {
        control.nao_manager.set_head(
            context.pose.get_look_at_absolute(&self.target),
            HeadJoints::fill(ROTATION_STIFFNESS),
            Priority::High,
        );

        control.walking_engine.request_stand();
    }
}
