use bevy::prelude::*;
use nidhogg::types::{FillExt, LegJoints};

use crate::motion::{
    energy_optimizer::{EnergyOptimizerExt, reset_joint_current_optimizer},
    walking_engine::{
        TargetFootPositions, TargetLegStiffness,
        config::WalkingEngineConfig,
        feet::FootPositions,
        hips::HipHeight,
        schedule::{Gait, WalkingEngineSet},
        step_context::StepContext,
    },
};

pub(super) struct StandGaitPlugin;

impl Plugin for StandGaitPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            generate_stand_gait
                .in_set(WalkingEngineSet::GenerateGait)
                .run_if(in_state(Gait::Standing)),
        );

        app.add_systems(OnEnter(Gait::Standing), reset_joint_current_optimizer);
        app.add_systems(OnExit(Gait::Standing), reset_joint_current_optimizer);
    }
}

#[derive(Debug, Deref)]
pub struct StandingHeight(f32);

impl StandingHeight {
    pub const MAX: Self = Self(0.26);

    #[must_use]
    pub fn new(height: f32) -> Self {
        Self(height)
    }
}

fn generate_stand_gait(
    mut commands: Commands,
    mut target: ResMut<TargetFootPositions>,
    mut hip_height: ResMut<HipHeight>,
    step_context: Res<StepContext>,
    mut target_stiffness: ResMut<TargetLegStiffness>,
    config: Res<WalkingEngineConfig>,
) {
    // always set the foot offsets to 0,0,0.
    **target = FootPositions::default();

    hip_height.request(
        *step_context
            .requested_standing_height
            .as_deref()
            .unwrap_or(&config.hip_height.walking_hip_height),
    );
    **target_stiffness = LegJoints::fill(config.standing_leg_stiffness);

    if !hip_height.is_adjusting() {
        commands.optimize_joint_currents();
    }
}
