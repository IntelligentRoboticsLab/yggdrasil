use bevy::prelude::*;
use nidhogg::types::{FillExt, LegJoints};

use crate::{
    kinematics::Kinematics,
    motion::walkv4::{
        config::WalkingEngineConfig,
        feet::FootPositions,
        hips::HipHeight,
        schedule::{Gait, GaitGeneration, MotionSet},
        TargetFootPositions, TargetLegStiffness,
    },
};

/// The leg stiffness value that's used when the robot is sitting.
/// Negative value means the motors will be turned off.
const SITTING_LEG_STIFFNESS: f32 = 0.0;
/// Threshold between two hip height values, when exceeded the values are considered different.
const HIP_HEIGHT_CHANGE_THRESHOLD: f32 = 0.003;
/// The number of cycles without any changes to the physical hip height
/// before the robot is considered to be stationary.
const NUM_CYCLES_BEFORE_CONSIDERED_STATIONARY: u32 = 120;

pub(super) struct SitGaitPlugin;

impl Plugin for SitGaitPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            GaitGeneration,
            (request_sit, generate_sit_gait)
                .chain()
                .in_set(MotionSet::GaitGeneration)
                .run_if(in_state(Gait::Sitting)),
        );
    }
}

fn request_sit(
    config: Res<WalkingEngineConfig>,
    mut hip_height: ResMut<HipHeight>,
    kinematics: Res<Kinematics>,
    mut last_hip_height: Local<f32>,
    mut no_change: Local<u32>,
    mut target_stiffness: ResMut<TargetLegStiffness>,
) {
    let actual_hip_height = kinematics.left_hip_height();

    if (actual_hip_height - *last_hip_height).abs() <= HIP_HEIGHT_CHANGE_THRESHOLD {
        *no_change += 1;
    } else {
        *no_change = 0;
    }

    *last_hip_height = actual_hip_height;

    if *no_change <= NUM_CYCLES_BEFORE_CONSIDERED_STATIONARY {
        let new_requested_hip_height =
            (actual_hip_height - 0.01).max(config.max_sitting_hip_height);

        hip_height.request(new_requested_hip_height);
        **target_stiffness = LegJoints::fill(config.leg_stiffness);
    } else {
        **target_stiffness = LegJoints::fill(SITTING_LEG_STIFFNESS);
    }
}

fn generate_sit_gait(mut target: ResMut<TargetFootPositions>) {
    // always set the foot offsets to 0,0,0.
    **target = FootPositions::default();
}
