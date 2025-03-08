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
        app.add_systems(OnEnter(Gait::Sitting), reset_balance_adjustment);
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
    left_leg: LeftLegJoints<f32>,
    right_leg: RightLegJoints<f32>,
}

impl BalanceAdjustment {
    fn init(config: &WalkingEngineConfig) -> Self {
        Self {
            filtered_gyro: ExponentialLpf::new(config.balancing.gyro_lpf_alpha),
            left_leg: LeftLegJoints::default(),
            right_leg: RightLegJoints::default(),
        }
    }

    /// Reset all adjustment values and prepare for new values.
    #[must_use]
    fn prepare(&mut self) -> &mut Self {
        self.left_leg = LeftLegJoints::default();
        self.right_leg = RightLegJoints::default();

        self
    }

    /// Apply the provided adjustment to the ankle pitch of the support foot.
    ///
    /// When swinging with the left foot, this will add the adjustment to the
    /// ankle pitch value of the right foot.
    pub fn adjust_ankle_pitch(&mut self, support_side: Side, amount: f32) -> &mut Self {
        match support_side {
            Side::Left => self.left_leg.ankle_pitch = amount,
            Side::Right => self.right_leg.ankle_pitch = amount,
        }

        self
    }

    /// Apply the provided target roll and pitch to the swing side.
    pub fn apply_foot_leveling(
        &mut self,
        swing_side: Side,
        target_roll: f32,
        target_pitch: f32,
    ) -> &mut Self {
        match swing_side {
            Side::Left => {
                self.left_leg.ankle_roll += target_roll;
                self.left_leg.ankle_pitch += target_pitch;
            }
            Side::Right => {
                self.right_leg.ankle_roll += target_roll;
                self.right_leg.ankle_pitch += target_pitch;
            }
        };

        self
    }

    pub fn apply_swing_leg_adjustments(
        &mut self,
        swing_side: Side,
        hip_pitch_override: f32,
        ankle_pitch_override: f32,
    ) -> &mut Self {
        match swing_side {
            Side::Left => {
                self.left_leg.hip_pitch += hip_pitch_override;
                self.left_leg.ankle_pitch += ankle_pitch_override;
            }
            Side::Right => {
                self.right_leg.hip_pitch += hip_pitch_override;
                self.right_leg.ankle_pitch += ankle_pitch_override;
            }
        };

        self
    }

    /// Apply this [`BalanceAdjustment`] to the provided leg joints.
    pub fn apply(
        &self,
        left_leg: &mut LeftLegJoints<f32>,
        right_leg: &mut RightLegJoints<f32>,
    ) -> &Self {
        left_leg.hip_yaw_pitch += self.left_leg.hip_yaw_pitch;
        left_leg.hip_pitch += self.left_leg.hip_pitch;
        left_leg.hip_roll += self.left_leg.hip_roll;
        left_leg.knee_pitch += self.left_leg.knee_pitch;
        left_leg.ankle_roll += self.left_leg.ankle_roll;
        left_leg.ankle_pitch += self.left_leg.ankle_pitch;

        right_leg.hip_pitch += self.right_leg.hip_pitch;
        right_leg.hip_roll += self.right_leg.hip_roll;
        right_leg.knee_pitch += self.right_leg.knee_pitch;
        right_leg.ankle_roll += self.right_leg.ankle_roll;
        right_leg.ankle_pitch += self.right_leg.ankle_pitch;

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
