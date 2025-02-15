use bevy::prelude::*;
use nidhogg::types::{LeftLegJoints, RightLegJoints};

use super::{
    config::WalkingEngineConfig,
    foot_support::FootSupportState,
    schedule::{Gait, WalkingEngineSet},
    Side,
};
use crate::sensor::{imu::IMUValues, low_pass_filter::ExponentialLpf};

/// Plugin for balancing the robot during [`MotionSet::Balancing`]
pub(super) struct BalancingPlugin;

impl Plugin for BalancingPlugin {
    fn build(&self, app: &mut App) {
        // ensure balance adjustment resource exists!
        app.add_systems(PostStartup, reset_balance_adjustment);
        app.add_systems(OnEnter(Gait::Standing), reset_balance_adjustment);
        app.add_systems(
            Update,
            (update_filtered_gyroscope, update_balance_adjustment)
                .chain()
                .in_set(WalkingEngineSet::Balance)
                .run_if(
                    in_state(Gait::Walking)
                        .or(in_state(Gait::Starting))
                        .or(in_state(Gait::Stopping)),
                ),
        );
    }
}

fn reset_balance_adjustment(mut commands: Commands, config: Res<WalkingEngineConfig>) {
    commands.insert_resource(BalanceAdjustment::init(&config));
}

fn update_filtered_gyroscope(
    mut balance_adjustment: ResMut<BalanceAdjustment>,
    imu: Res<IMUValues>,
) {
    balance_adjustment.filtered_gyro.update(imu.gyroscope);
}

/// Resource that stores balance adjustment values for the walking engine.
///
/// # Note
///
/// This only applies a counter rotation to the ankle pitch of the support foot, but this can
/// be extended further to the roll, and other joints as well.
#[derive(Resource, Debug, Clone)]
pub struct BalanceAdjustment {
    filtered_gyro: ExponentialLpf<3>,
    left_ankle_pitch: f32,
    right_ankle_pitch: f32,
}

impl BalanceAdjustment {
    fn init(config: &WalkingEngineConfig) -> Self {
        Self {
            filtered_gyro: ExponentialLpf::new(config.balancing.gyro_lpf_alpha),
            left_ankle_pitch: 0.,
            right_ankle_pitch: 0.,
        }
    }
    /// Reset all adjustment values and prepare for new values.
    #[must_use]
    fn prepare(&mut self) -> &mut Self {
        self.left_ankle_pitch = 0f32;
        self.right_ankle_pitch = 0f32;

        self
    }

    /// Apply the provided adjustment to the ankle pitch of the support foot.
    ///
    /// When swinging with the left foot, this will add the adjustment to the
    /// ankle pitch value of the right foot.
    fn adjust_ankle_pitch(&mut self, support_side: Side, amount: f32) -> &mut Self {
        match support_side {
            Side::Left => self.left_ankle_pitch = amount,
            Side::Right => self.right_ankle_pitch = amount,
        }

        self
    }

    /// Apply this [`BalanceAdjustment`] to the provided leg joints.
    pub fn apply(
        &self,
        left_leg: &mut LeftLegJoints<f32>,
        right_leg: &mut RightLegJoints<f32>,
    ) -> &Self {
        left_leg.ankle_pitch += self.left_ankle_pitch;
        right_leg.ankle_pitch += self.right_ankle_pitch;

        self
    }
}

fn update_balance_adjustment(
    mut balance_adjustment: ResMut<BalanceAdjustment>,
    foot_support: Res<FootSupportState>,
    config: Res<WalkingEngineConfig>,
) {
    let ankle_pitch_adjustment =
        balance_adjustment.filtered_gyro.state().y * config.balancing.filtered_gyro_y_multiplier;

    balance_adjustment
        .prepare()
        .adjust_ankle_pitch(foot_support.support_side(), ankle_pitch_adjustment);
}
