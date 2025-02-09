use bevy::prelude::*;

use nidhogg::types::{ArmJoints, FillExt, HeadJoints, JointArray, LegJoints};

use crate::{
    behavior::engine::{in_behavior, Behavior, BehaviorState},
    motion::walkv4::step_manager::StepManager,
    nao::{NaoManager, Priority, RobotInfo},
};

const DEFAULT_PASSIVE_STIFFNESS: f32 = 0.8;
// This should run with priority over the walking engine.
const DEFAULT_PASSIVE_PRIORITY: Priority = Priority::High;

pub struct StartUpBehaviorPlugin;

impl Plugin for StartUpBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, startup.run_if(in_behavior::<StartUp>));
    }
}

/// This is the startup behavior of the robot.
/// In this state the robot does nothing and retains its previous position.
#[derive(Resource)]
pub struct StartUp;

impl Behavior for StartUp {
    const STATE: BehaviorState = BehaviorState::StartUp;
}

pub fn startup(
    robot_info: Res<RobotInfo>,
    mut step_manager: ResMut<StepManager>,
    mut nao_manager: ResMut<NaoManager>,
) {
    step_manager.request_stand();
    set_initial_joint_values(&robot_info.initial_joint_positions, &mut nao_manager);
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
