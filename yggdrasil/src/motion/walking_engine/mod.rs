use balancing::BalanceAdjustment;
use bevy::prelude::*;
use config::WalkingEngineConfig;
use feet::FootPositions;
use hips::HipHeight;
use nidhogg::types::{ArmJoints, FillExt, LeftLegJoints, LegJoints, RightLegJoints};

use crate::{
    kinematics,
    motion::walking_engine::config::KickingConfig,
    nao::{NaoManager, Priority},
    prelude::ConfigExt,
};

mod arm_swing;
mod balancing;
pub mod config;
pub mod feet;
pub mod foot_support;
mod gait;
pub mod hips;
mod schedule;
pub mod smoothing;
pub mod step;
pub mod step_context;

pub use gait::StandingHeight;
pub use schedule::{Gait, WalkingEngineSet};

pub struct WalkingEnginePlugin;

impl Plugin for WalkingEnginePlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<WalkingEngineConfig>();
        app.init_config::<KickingConfig>();
        app.init_resource::<TargetFootPositions>();
        app.init_resource::<TargetLegStiffness>();
        app.add_event::<FootSwitchedEvent>();
        app.add_plugins((
            schedule::WalkingEngineSchedulePlugin,
            step_context::StepContextPlugin,
            hips::HipHeightPlugin,
            gait::GaitPlugin,
            balancing::BalancingPlugin,
            foot_support::FootSupportPlugin,
        ));

        app.add_systems(Update, finalize.in_set(WalkingEngineSet::Finalize));
    }
}

#[derive(Resource, Debug, Default, Clone, Copy)]
pub struct RequestedStep {
    pub forward: f32,
    pub left: f32,
    pub turn: f32,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    #[default]
    Left,
    Right,
}

impl Side {
    #[must_use]
    pub fn opposite(self) -> Self {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

/// Event which is fired when the support foot has switched.
#[derive(Event, Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct FootSwitchedEvent {
    /// The new support side.
    pub new_support: Side,
    /// The new swing side.
    pub new_swing: Side,
}

/// Resource containing the currently requested [`FootPositions`].
#[derive(Debug, Default, Clone, Resource, Deref, DerefMut)]
pub struct TargetFootPositions(FootPositions);

impl TargetFootPositions {
    /// Compute the leg angles for the target foot positions.
    #[must_use]
    pub fn leg_angles(
        &self,
        hip_height: f32,
        torso_offset: f32,
    ) -> (LeftLegJoints<f32>, RightLegJoints<f32>) {
        kinematics::inverse::leg_angles(&self.0, hip_height, torso_offset)
    }
}

/// Resource containing the currently requested leg stiffness.
#[derive(Debug, Default, Clone, Resource, Deref, DerefMut)]
pub struct TargetLegStiffness(LegJoints<f32>);

fn finalize(
    mut nao: ResMut<NaoManager>,
    hip_height: Res<HipHeight>,
    target_foot_positions: Res<TargetFootPositions>,
    target_leg_stiffness: Res<TargetLegStiffness>,
    balance_adjustment: Res<BalanceAdjustment>,
    motion_state: Res<State<Gait>>,
    config: Res<WalkingEngineConfig>,
) {
    let (mut left_leg, mut right_leg) =
        target_foot_positions.leg_angles(hip_height.current(), config.torso_offset);
    balance_adjustment.apply(&mut left_leg, &mut right_leg);

    let left_arm = arm_swing::swinging_arm(
        left_leg.hip_roll,
        target_foot_positions.right.translation.x,
        true,
    );
    let right_arm = arm_swing::swinging_arm(
        -right_leg.hip_roll,
        target_foot_positions.left.translation.x,
        false,
    );

    let leg_positions = LegJoints::builder()
        .left_leg(left_leg)
        .right_leg(right_leg)
        .build();

    let leg_stiffness = LegJoints::builder()
        .left_leg(target_leg_stiffness.left_leg.clone())
        .right_leg(target_leg_stiffness.right_leg.clone())
        .build();

    if *motion_state == Gait::Walking {
        let arm_positions = ArmJoints::builder()
            .left_arm(left_arm)
            .right_arm(right_arm)
            .build();

        nao.set_arms(
            arm_positions,
            ArmJoints::fill(config.arm_stiffness),
            Priority::Medium,
        );
    } else {
        nao.set_arms(
            ArmJoints::builder()
                .left_arm(arm_swing::swinging_arm(0.0, 0.0, true))
                .right_arm(arm_swing::swinging_arm(0.0, 0.0, false))
                .build(),
            ArmJoints::fill(0.0),
            Priority::Medium,
        );
    }

    nao.set_legs(leg_positions, leg_stiffness, Priority::Medium);
}
