use balancing::BalanceAdjustment;
use bevy::prelude::*;
use config::WalkingEngineConfig;
use feet::FootPositions;
use hips::HipHeight;
use nidhogg::types::{ArmJoints, FillExt, LeftLegJoints, LegJoints, RightLegJoints};
use scheduling::{Gait, MotionSet};
use step::Step;
use step_manager::StepManager;

use crate::{
    kinematics,
    nao::{NaoManager, Priority},
    prelude::ConfigExt,
    sensor::button::{ChestButton, HeadButtons},
};

mod arm_swing;
mod balancing;
pub mod config;
pub mod feet;
mod foot_support;
mod gait;
pub mod hips;
mod scheduling;
mod smoothing;
pub mod step;
pub mod step_manager;

// TODO: dynamically set this
/// The offset of the torso w.r.t. the hips.
pub const TORSO_OFFSET: f32 = 0.015;

pub struct Walkv4EnginePlugin;

impl Plugin for Walkv4EnginePlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<WalkingEngineConfig>();
        app.init_resource::<SwingFoot>();
        app.init_resource::<TargetFootPositions>();
        app.init_resource::<TargetLegStiffness>();
        app.add_event::<FootSwitchedEvent>();
        app.add_plugins((
            scheduling::MotionSchedulePlugin,
            step_manager::StepManagerPlugin,
            hips::HipHeightPlugin,
            gait::GaitPlugins,
            balancing::BalancingPlugin,
            foot_support::FootSupportPlugin,
        ));

        app.add_systems(
            Update,
            (
                switch_state
                    .in_set(MotionSet::StepPlanning)
                    .before(step_manager::sync_gait_request),
                finalize.in_set(MotionSet::Finalize),
            ),
        );
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

/// Resource containing the current swing foot of the walking engine.
#[derive(Resource, Clone, Copy, Debug, Default, Deref, DerefMut)]
pub struct SwingFoot(Side);

impl SwingFoot {
    /// Get the side of the support foot.
    #[must_use]
    pub fn support(&self) -> Side {
        self.opposite()
    }

    /// Get the side of the swing foot.
    #[must_use]
    pub fn swing(&self) -> Side {
        self.0
    }
}

#[derive(Event, Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct FootSwitchedEvent(pub Side);

/// Resource containing the currently requested [`FootPositions`].
#[derive(Debug, Default, Clone, Resource, Deref, DerefMut)]
pub struct TargetFootPositions(FootPositions);

/// Resource containing the currently requested leg stiffness.
#[derive(Debug, Default, Clone, Resource, Deref, DerefMut)]
pub struct TargetLegStiffness(LegJoints<f32>);

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

fn switch_state(
    current_state: Res<State<Gait>>,
    mut step_manager: ResMut<StepManager>,
    chest_button: Res<ChestButton>,
    head_buttons: Res<HeadButtons>,
) {
    let chest_tapped = chest_button.state.is_tapped();
    let head_tapped = head_buttons.all_pressed();

    if chest_tapped {
        if *current_state == Gait::Sitting {
            step_manager.request_stand();
        } else if *current_state == Gait::Standing {
            step_manager.request_walk(Step {
                forward: 0.05,
                left: 0.0,
                turn: 0.0,
            });
        }
    }

    if head_tapped {
        step_manager.request_sit();
    }
}

fn finalize(
    mut nao: ResMut<NaoManager>,
    hip_height: Res<HipHeight>,
    target_foot_positions: Res<TargetFootPositions>,
    target_leg_stiffness: Res<TargetLegStiffness>,
    balance_adjustment: Res<BalanceAdjustment>,
    motion_state: Res<State<Gait>>,
) {
    let (mut left_leg, mut right_leg) =
        target_foot_positions.leg_angles(hip_height.current(), TORSO_OFFSET);
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

    let arm_positions = ArmJoints::builder()
        .left_arm(left_arm)
        .right_arm(right_arm)
        .build();

    let leg_positions = LegJoints::builder()
        .left_leg(left_leg)
        .right_leg(right_leg)
        .build();

    let leg_stiffness = LegJoints::builder()
        .left_leg(target_leg_stiffness.left_leg.clone())
        .right_leg(target_leg_stiffness.right_leg.clone())
        .build();

    if *motion_state == Gait::Walking {
        nao.set_arms(arm_positions, ArmJoints::fill(0.8), Priority::Medium);
    } else {
        nao.set_arms(
            ArmJoints::builder()
                .left_arm(arm_swing::swinging_arm(0.0, 0.0, true))
                .right_arm(arm_swing::swinging_arm(0.0, 0.0, false))
                .build(),
            ArmJoints::fill(0.8),
            Priority::Medium,
        );
    }

    nao.set_legs(leg_positions, leg_stiffness, Priority::Medium);
}
