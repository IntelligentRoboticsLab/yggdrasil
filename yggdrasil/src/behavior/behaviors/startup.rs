use crate::{
    behavior::engine::{Behavior, Context},
    motion::keyframe::MotionManager,
    motion::step_planner::StepPlanner,
    motion::walk::engine::WalkingEngine,
    nao::manager::{NaoManager, Priority},
};
use nidhogg::types::{ArmJoints, FillExt, HeadJoints, JointArray, LegJoints};

const DEFAULT_PASSIVE_STIFFNESS: f32 = 0.8;
const DEFAULT_PASSIVE_PRIORITY: Priority = Priority::Medium;

/// This is the startup behavior of the robot.
/// In this state the robot does nothing and retains its previous position.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub struct StartUp;

impl Behavior for StartUp {
    fn execute(
        &mut self,
        context: Context,
        nao_manager: &mut NaoManager,
        _walking_engine: &mut WalkingEngine,
        _: &mut MotionManager,
        _step_planner: &mut StepPlanner,
    ) {
        set_initial_joint_values(&context.robot_info.initial_joint_positions, nao_manager);
    }
}

fn set_initial_joint_values(
    initial_joint_positions: &JointArray<f32>,
    nao_manager: &mut NaoManager,
) {
    nao_manager.set_all(
        initial_joint_positions.clone(),
        HeadJoints::fill(DEFAULT_PASSIVE_STIFFNESS),
        ArmJoints::fill(DEFAULT_PASSIVE_STIFFNESS),
        LegJoints::fill(DEFAULT_PASSIVE_STIFFNESS),
        DEFAULT_PASSIVE_PRIORITY,
    );
}
