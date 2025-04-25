use bevy::prelude::*;
use nidhogg::{
    NaoState,
    types::{FillExt, JointArray, LegJoints},
};

use crate::{
    kinematics::Kinematics,
    motion::{
        energy_optimizer::EnergyOptimizerExt,
        walking_engine::{
            TargetFootPositions, TargetLegStiffness,
            config::WalkingEngineConfig,
            feet::FootPositions,
            hips::HipHeight,
            schedule::{Gait, WalkingEngineSet},
        },
    },
    nao::{NaoManager, Priority},
};

pub(super) struct SitGaitPlugin;

impl Plugin for SitGaitPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (request_sit, generate_sit_gait)
                .chain()
                .in_set(WalkingEngineSet::GenerateGait)
                .run_if(in_state(Gait::Sitting)),
        );
    }
}

fn request_sit(
    mut commands: Commands,
    config: Res<WalkingEngineConfig>,
    mut hip_height: ResMut<HipHeight>,
    kinematics: Res<Kinematics>,
    mut target_stiffness: ResMut<TargetLegStiffness>,
    (nao_state, mut manager): (Res<NaoState>, ResMut<NaoManager>),
    mut sitting_pose: Local<Option<JointArray<f32>>>,
) {
    let actual_hip_height = kinematics.left_hip_height();

    if actual_hip_height <= config.hip_height.max_sitting_hip_height {
        // Already at sitting height, optimize and don't change stiffness
        if !hip_height.is_adjusting() {
            let mut sitting_stiffness = LegJoints::fill(config.sitting_leg_stiffness);
            sitting_stiffness.left_leg.hip_pitch = 0.1;
            sitting_stiffness.right_leg.hip_pitch = 0.1;

            let pose = sitting_pose.get_or_insert(nao_state.position.clone());
            manager.set_legs(
                LegJoints::builder()
                    .left_leg(pose.left_leg_joints())
                    .right_leg(pose.right_leg_joints())
                    .build(),
                sitting_stiffness,
                Priority::High,
            );

            commands.optimize_joint_currents();

            return;
        }
    } else {
        // still too high, keep lowering
        let new_hip_height =
            (actual_hip_height - 0.01).max(config.hip_height.max_sitting_hip_height);
        hip_height.request(new_hip_height);
        **target_stiffness = LegJoints::fill(config.standing_leg_stiffness);
    }

    commands.reset_joint_current_optimizer();
    *sitting_pose = None;
}

fn generate_sit_gait(mut target: ResMut<TargetFootPositions>) {
    // Set foot offsets to 0,0,0
    **target = FootPositions::default();
}
