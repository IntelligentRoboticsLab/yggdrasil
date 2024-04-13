use nidhogg::types::{color::f32::RED, FillExt, HeadJoints};

use crate::{
    behavior::engine::{Behavior, Context},
    nao::manager::{NaoManager, Priority},
    walk::engine::WalkingEngine,
};

const PENALIZED_HEAD_STIFFNESS: f32 = 0.3;

/// During a match the chest button is pressed before starting a match.
/// Once this is done, the robots are placed at the edge of the field from
/// which they will walk to their `Ready` positions.
///
/// This is the behaviour of the robot once the chest button is pressed.
/// In this state the robot will stand up straight and look at the middle
/// circle to make it easier to place the robot in the correct position.
#[derive(Copy, Clone, Debug, Default)]
pub struct Penalized;

impl Behavior for Penalized {
    fn execute(
        &mut self,
        _context: Context,
        nao_manager: &mut NaoManager,
        walking_engine: &mut WalkingEngine,
    ) {
        walking_engine.request_stand();

        let head_joints = HeadJoints::fill(0.0);
        let head_stiffness = HeadJoints::fill(PENALIZED_HEAD_STIFFNESS);

        nao_manager.set_head(head_joints, head_stiffness, Priority::High);
        nao_manager.set_chest_led(RED, Priority::High);
    }
}
