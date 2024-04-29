use crate::{
    behavior::engine::{Behavior, Context},
    motion::{motion_manager::MotionManager, motion_types::MotionType, step_planner::StepPlanner},
    nao::manager::{NaoManager, Priority},
    walk::engine::WalkingEngine,
};

/// Behavior used for making the robot do a lil' floss dance.
///
/// The floss motion uses the robots torso as a counterweight
/// to stabilize itself, but this stabilization can fail when
/// the motion continues for a long time.
///
/// # Notes
/// - Currently this behavior is very dangerous to use against
///   other teams since the mental damage doing a floss dance
///   after scoring will do, is tremendous.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Floss;

impl Behavior for Floss {
    fn execute(
        &mut self,
        _context: Context,
        _nao_manager: &mut NaoManager,
        _walking_engine: &mut WalkingEngine,
        motion_manager: &mut MotionManager,
        _step_planner: &mut StepPlanner,
    ) {
        motion_manager.start_new_motion(MotionType::Floss, Priority::Medium);
    }
}
