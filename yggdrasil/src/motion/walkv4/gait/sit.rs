use std::time::Instant;

use bevy::prelude::*;
use nidhogg::types::{FillExt, LegJoints};

use crate::{
    kinematics::Kinematics,
    motion::walkv4::{
        config::WalkingEngineConfig,
        feet::FootPositions,
        hips::HipHeight,
        schedule::{Gait, GaitGeneration, WalkingEngineSet},
        TargetFootPositions, TargetLegStiffness,
    },
    sensor::fsr::Contacts,
};

pub(super) struct SitGaitPlugin;

impl Plugin for SitGaitPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            GaitGeneration,
            (request_sit, generate_sit_gait)
                .chain()
                .in_set(WalkingEngineSet::GenerateGait)
                .run_if(in_state(Gait::Sitting)),
        );
    }
}

fn request_sit(
    config: Res<WalkingEngineConfig>,
    mut hip_height: ResMut<HipHeight>,
    kinematics: Res<Kinematics>,
    mut last_hip_height: Local<f32>,
    mut stable_since: Local<Option<Instant>>,
    mut target_stiffness: ResMut<TargetLegStiffness>,
    (mut captured_hip_height, contacts): (Local<Option<f32>>, Res<Contacts>),
) {
    let actual_hip_height = kinematics.left_hip_height();

    let has_changed =
        (actual_hip_height - *last_hip_height).abs() <= config.hip_height.change_threshold;
    *last_hip_height = actual_hip_height;

    if has_changed {
        *stable_since = None;
    } else if stable_since.is_none() {
        *stable_since = Some(Instant::now());
    }

    // if the robot is considered stable for atleast the configured timeout, we set the stiffness to the configured
    // sitting leg stiffness.
    if stable_since.is_some_and(|timestamp| timestamp.elapsed() >= config.stable_sitting_timeout) {
        **target_stiffness = LegJoints::fill(config.sitting_leg_stiffness);

        // capture hip height
        *captured_hip_height =
            Some(actual_hip_height.max(config.hip_height.max_sitting_hip_height));
    } else {
        if !contacts.ground {
            // robot got picked up, and we have captured sitting height before, so we can set it directly.
            if let Some(captured) = *captured_hip_height {
                hip_height.request(captured);
                **target_stiffness = LegJoints::fill(config.walking_leg_stiffness);
                return;
            }
        }

        let new_requested_hip_height =
            (actual_hip_height - 0.01).max(config.hip_height.max_sitting_hip_height);

        hip_height.request(new_requested_hip_height);
        **target_stiffness = LegJoints::fill(config.walking_leg_stiffness);
    }
}

fn generate_sit_gait(mut target: ResMut<TargetFootPositions>) {
    // always set the foot offsets to 0,0,0.
    **target = FootPositions::default();
}
