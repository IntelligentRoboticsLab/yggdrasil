use bevy::prelude::*;
use feet::FootPositions;
use nidhogg::types::{LeftLegJoints, RightLegJoints};

mod balancing;
mod feet;
mod gait;
mod hips;
mod mod_walk;
mod scheduling;
mod step;
mod support_foot;

pub struct Walkv4EnginePlugin;

impl Plugin for Walkv4EnginePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SwingFoot>();
        app.init_resource::<TargetFootPositions>();
        app.add_plugins((
            scheduling::MotionSchedulePlugin,
            gait::GaitPlugins,
            balancing::BalancingPlugin,
        ));
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

/// Resource containing the currently requested [`FootPositions`], and the balance adjustment.
#[derive(Debug, Default, Clone, Resource)]
pub struct TargetFootPositions {
    foot_positions: FootPositions,
    balance_adjustment: f32,
}

impl TargetFootPositions {
    /// Apply the [`FootPositions`] from the gait generator.
    pub fn apply_gait(&mut self, foot_positions: FootPositions) {
        self.foot_positions = foot_positions;
    }

    /// Apply the balance adjustment value to the current [`TargetFootPositions`].
    pub fn apply_balance_adjustment(&mut self, balance_adjustment: f32) {
        self.balance_adjustment = balance_adjustment;
    }

    pub fn leg_angles(&self) -> (LeftLegJoints<f32>, RightLegJoints<f32>) {
        let FootPositions { left, right } = self.foot_positions;

        let left_foot_offset = FootOffset {
            forward: left.translation.x,
            left: left.translation.y - ROBOT_TO_LEFT_PELVIS.y,
            turn: 0.,
            lift: left_lift,
            hip_height,
            ..Default::default()
        };

        let right_foot_offset = FootOffset {
            forward: right.translation.x,
            left: right.translation.y - ROBOT_TO_RIGHT_PELVIS.y,
            turn: 0.,
            lift: right_lift,
            hip_height,
            ..Default::default()
        };

        let (mut left, mut right) =
            kinematics::inverse::leg_angles(&left_foot_offset, &right_foot_offset, TORSO_OFFSET);
    }
}
