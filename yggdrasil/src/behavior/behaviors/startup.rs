use crate::{
    behavior::engine::{Behavior, Context},
    nao::manager::{NaoManager, Priority},
    walk::{self, engine::WalkingEngine},
};
use nidhogg::types::{ArmJoints, FillExt, HeadJoints, JointArray, LegJoints};

const DEFAULT_PASSIVE_STIFFNESS: f32 = 0.8;
const DEFAULT_PASSIVE_PRIORITY: Priority = Priority::Medium;

/// This is the default behavior of the robot.
/// In this state the robot does nothing and retains its previous position.
/// In this state the robot has a blue right eye.
#[derive(Copy, Clone, Debug, Default)]
pub struct StartUp;

impl Behavior for StartUp {
    fn execute(
        &mut self,
        context: Context,
        nao_manager: &mut NaoManager,
        _walking_engine: &mut WalkingEngine,
    ) {
        set_initial_joint_values(&context.robot_info.initial_joint_positions, nao_manager);
    }
}

fn set_initial_joint_values(
    initial_joint_positions: &JointArray<f32>,
    nao_manager: &mut NaoManager,
) {
    nao_manager.set_head(
        initial_joint_positions.head_joints(),
        HeadJoints::fill(DEFAULT_PASSIVE_STIFFNESS),
        DEFAULT_PASSIVE_PRIORITY,
    );

    nao_manager.set_arms(
        initial_joint_positions.arm_joints(),
        ArmJoints::fill(DEFAULT_PASSIVE_STIFFNESS),
        DEFAULT_PASSIVE_PRIORITY,
    );
    println!("We Startuping");

    nao_manager.set_legs(
        initial_joint_positions.leg_joints(),
        LegJoints::fill(DEFAULT_PASSIVE_STIFFNESS),
        DEFAULT_PASSIVE_PRIORITY,
    );
}
