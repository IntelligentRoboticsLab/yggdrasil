use crate::{
    behavior::engine::{Behavior, Context},
    motion::step_planner::StepPlanner,
    nao::manager::{NaoManager, Priority},
    walk::engine::WalkingEngine,
};
use nalgebra::Point2;
use nidhogg::types::{FillExt, HeadJoints};

/// During a match the chest button is pressed before starting a match.
/// Once this is done, the robots are placed at the edge of the field from
/// which they will walk to their `Ready` positions.
///
/// This is the behaviour of the robot once the chest button is pressed.
/// In this state the robot will stand up straight and look at the middle
/// circle to make it easier to place the robot in the correct position.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct Initial;

impl Behavior for Initial {
    fn execute(
        &mut self,
        context: Context,
        nao_manager: &mut NaoManager,
        walking_engine: &mut WalkingEngine,
        _step_planner: &mut StepPlanner,
    ) {
        nao_manager.set_head(
            context.pose.get_look_at_absolute(&Point2::origin()),
            HeadJoints::fill(1.0),
            Priority::High,
        );

        walking_engine.request_stand();
    }
}
