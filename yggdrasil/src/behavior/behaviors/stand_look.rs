use crate::{
    behavior::engine::{Behavior, Context},
    motion::step_planner::StepPlanner,
    config::layout::{RobotPosition, WorldPosition},
    motion::step_planning::StepPlanner,
    nao::manager::{NaoManager, Priority},
    walk::engine::WalkingEngine,
};
use nalgebra::Point2;
use nidhogg::types::{FillExt, HeadJoints};

const HEAD_STIFFNESS: f32 = 0.4;

/// During a match the chest button is pressed before starting a match.
/// Once this is done, the robots are placed at the edge of the field from
/// which they will walk to their `Ready` positions.
///
/// This is the behaviour of the robot once the chest button is pressed.
/// In this state the robot will stand up straight and look at the middle
/// circle to make it easier to place the robot in the correct position.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct StandingLookAt {
    pub target: WorldPosition,
}

impl Behavior for StandingLookAt {
    fn execute(
        &mut self,
        context: Context,
        nao_manager: &mut NaoManager,
        walking_engine: &mut WalkingEngine,
        _step_planner: &mut StepPlanner,
    ) {
        nao_manager.set_head(
            context.pose.get_look_at_absolute(&Point2::origin()),
            HeadJoints::fill(HEAD_STIFFNESS),
            Priority::High,
        );

        walking_engine.request_stand();
    }
}
