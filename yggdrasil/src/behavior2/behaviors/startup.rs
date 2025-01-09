use bevy::prelude::*;

use nidhogg::types::{ArmJoints, FillExt, HeadJoints, JointArray, LegJoints};

use crate::{
    behavior2::engine::{Behavior, BehaviorState},
    impl_behavior,
    motion::walk::engine::WalkingEngine,
    nao::{NaoManager, Priority, RobotInfo},
};

pub struct StartUpBehaviorPlugin;

impl Plugin for StartUpBehaviorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, startup.run_if(in_state(BehaviorState::StartUp)));
    }
}

/// This is the startup behavior of the robot.
/// In this state the robot does nothing and retains its previous position.
#[derive(Resource)]
pub struct StartUp;

impl_behavior!(StartUp, StartUp);

const DEFAULT_PASSIVE_STIFFNESS: f32 = 0.8;
// This should run with priority over the walking engine.
const DEFAULT_PASSIVE_PRIORITY: Priority = Priority::High;

pub fn startup(
    robot_info: Res<RobotInfo>,
    mut walking_engine: ResMut<WalkingEngine>,
    mut nao_manager: ResMut<NaoManager>,
) {
    walking_engine.request_stand();
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
