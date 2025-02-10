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
    sensor::orientation::RobotOrientation,
};

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
    orientation: Res<RobotOrientation>,
    mut target_stiffness: ResMut<TargetLegStiffness>,
) {
    let actual_hip_height = kinematics.left_hip_height();

    let has_changed =
        (actual_hip_height - *last_hip_height).abs() <= config.hip_height.change_threshold;
    *last_hip_height = actual_hip_height;

    if !has_changed && orientation.is_resting() {
        let new_requested_hip_height =
            (actual_hip_height - 0.01).max(config.hip_height.max_sitting_hip_height);

        hip_height.request(new_requested_hip_height);
        **target_stiffness = LegJoints::fill(config.walking_leg_stiffness);
    } else {
        **target_stiffness = LegJoints::fill(config.sitting_leg_stiffness);
    }
}

fn generate_sit_gait(mut target: ResMut<TargetFootPositions>) {
    // always set the foot offsets to 0,0,0.
    **target = FootPositions::default();
}
