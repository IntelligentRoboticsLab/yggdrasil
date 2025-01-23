use balancing::BalanceAdjustment;
use bevy::prelude::*;
use config::WalkingEngineConfig;
use feet::FootPositions;
use hips::HipHeight;
use nidhogg::types::{FillExt, LeftLegJoints, LegJoints, RightLegJoints};
use scheduling::{MotionSet, MotionState};

use crate::{
    kinematics,
    nao::{NaoManager, Priority},
    prelude::ConfigExt,
    sensor::button::{ChestButton, HeadButtons},
};

mod balancing;
pub mod config;
mod feet;
mod gait;
pub mod hips;
mod scheduling;
mod step;

// TODO: dynamically set this
/// The offset of the torso w.r.t. the hips.
pub const TORSO_OFFSET: f32 = 0.025;

pub struct Walkv4EnginePlugin;

impl Plugin for Walkv4EnginePlugin {
    fn build(&self, app: &mut App) {
        app.init_config::<WalkingEngineConfig>();
        app.init_resource::<SwingFoot>();
        app.init_resource::<TargetFootPositions>();
        app.add_plugins((
            scheduling::MotionSchedulePlugin,
            hips::HipHeightPlugin,
            gait::GaitPlugins,
            balancing::BalancingPlugin,
        ));

        app.add_systems(
            Update,
            (
                switch_state.in_set(MotionSet::StepPlanning),
                finalize.in_set(MotionSet::Finalize),
            ),
        );
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
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

/// Resource containing the currently requested [`FootPositions`], and the balance adjustment.
#[derive(Debug, Default, Clone, Resource, Deref, DerefMut)]
pub struct TargetFootPositions(FootPositions);

impl TargetFootPositions {
    /// Compute the leg angles for the target foot positions.
    pub fn leg_angles(
        &self,
        hip_height: f32,
        torso_offset: f32,
    ) -> (LeftLegJoints<f32>, RightLegJoints<f32>) {
        let foot_offsets = self.to_offsets(hip_height);
        // info!(?foot_offsets, "target foot offsets");
        kinematics::inverse::leg_angles(&foot_offsets.left, &foot_offsets.right, torso_offset)
    }
}

fn switch_state(
    current_state: Res<State<MotionState>>,
    mut next_state: ResMut<NextState<MotionState>>,
    chest_button: Res<ChestButton>,
    head_buttons: Res<HeadButtons>,
) {
    info!(?current_state, "\n\n\ncurrent state");
    let chest_tapped = chest_button.state.is_tapped();
    let head_tapped = head_buttons.all_pressed();

    if chest_tapped {
        if *current_state == MotionState::Sitting {
            next_state.set(MotionState::Standing);
        } else if *current_state == MotionState::Standing {
            next_state.set(MotionState::Walking);
        }
    }

    if head_tapped {
        next_state.set(MotionState::Sitting);
    }
}

fn finalize(
    mut nao: ResMut<NaoManager>,
    config: Res<WalkingEngineConfig>,
    hip_height: Res<HipHeight>,
    target_foot_positions: Res<TargetFootPositions>,
    balance_adjustment: Res<BalanceAdjustment>,
) {
    let (mut left_leg, mut right_leg) =
        target_foot_positions.leg_angles(hip_height.current(), TORSO_OFFSET);
    balance_adjustment.apply(&mut left_leg, &mut right_leg);

    let leg_positions = LegJoints::builder()
        .left_leg(left_leg)
        .right_leg(right_leg)
        .build();
    let leg_stiffness = LegJoints::builder()
        .left_leg(LeftLegJoints::fill(config.leg_stiffness))
        .right_leg(RightLegJoints::fill(config.leg_stiffness))
        .build();

    nao.set_legs(leg_positions, leg_stiffness, Priority::Medium);
}
