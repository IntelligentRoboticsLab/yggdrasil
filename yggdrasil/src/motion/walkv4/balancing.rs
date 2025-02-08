use bevy::prelude::*;
use nidhogg::types::{LeftLegJoints, RightLegJoints};

use super::{
    config::WalkingEngineConfig,
    schedule::{Balancing, Gait, MotionSet},
    Side, SwingFoot,
};
use crate::sensor::{imu::IMUValues, low_pass_filter::LowPassFilter};

// TODO: Make config value
/// The cut-off frequency for the butterworth lowpass filter used for the gyroscope values.
/// Higher values means that the filtered gyroscope value responds to changes quicker,
/// and lower values mean that it responds slower.
const FILTERED_GYRO_OMEGA: f32 = 0.055;

/// Plugin for balancing the robot during [`MotionSet::Balancing`]
pub(super) struct BalancingPlugin;

impl Plugin for BalancingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BalanceAdjustment>();

        app.add_systems(OnEnter(Gait::Standing), reset_balance_adjustment);
        app.add_systems(
            Balancing,
            (update_filtered_gyroscope, update_balance_adjustment)
                .chain()
                .in_set(MotionSet::Balancing)
                .run_if(in_state(Gait::Walking)),
        );
    }
}

fn reset_balance_adjustment(mut commands: Commands) {
    commands.insert_resource(BalanceAdjustment::default());
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
    filtered_gyro: LowPassFilter<3>,
    left_ankle_pitch: f32,
    right_ankle_pitch: f32,
}

impl Default for BalanceAdjustment {
    fn default() -> Self {
        Self {
            filtered_gyro: LowPassFilter::new(FILTERED_GYRO_OMEGA),
            left_ankle_pitch: 0f32,
            right_ankle_pitch: 0f32,
        }
    }
}

impl BalanceAdjustment {
    /// Reset all adjustment values and prepare for new values.
    #[must_use]
    fn prepare(&mut self) -> &mut Self {
        self.left_ankle_pitch = 0f32;
        self.right_ankle_pitch = 0f32;

        self
    }

    /// Apply the provided adjustment to the correct ankle pitch.
    ///
    /// When swinging with the left foot, this will add the adjustment to the
    /// ankle pitch value of the right foot.
    // TODO: Add figure here
    fn adjust_ankle_pitch(&mut self, swing_side: Side, amount: f32) -> &mut Self {
        match swing_side {
            Side::Left => self.right_ankle_pitch = amount,
            Side::Right => self.left_ankle_pitch = amount,
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
    swing_foot: Res<SwingFoot>,
    config: Res<WalkingEngineConfig>,
) {
    let adjustment =
        balance_adjustment.filtered_gyro.state().y * config.balancing.filtered_gyro_y_multiplier;

    balance_adjustment
        .prepare()
        .adjust_ankle_pitch(**swing_foot, adjustment);
}
