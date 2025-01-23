use bevy::prelude::*;

use super::{scheduling::MotionSet, TargetFootPositions};
use crate::{
    motion::walk::WalkingEngineConfig,
    sensor::{imu::IMUValues, low_pass_filter::LowPassFilter},
};

// TODO: Make config value
const FILTERED_GYRO_OMEGA: f32 = 0.115;

/// Plugin for balancing the robot during [`MotionSet::Balancing`]
pub(super) struct BalancingPlugin;

impl Plugin for BalancingPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(FilteredGyroscope(LowPassFilter::new(FILTERED_GYRO_OMEGA)));
        app.add_systems(
            Update,
            (update_filtered_gyroscope, update_balance_adjustment)
                .chain()
                .in_set(MotionSet::Balancing),
        );
    }
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct FilteredGyroscope(LowPassFilter<3>);

fn update_filtered_gyroscope(mut filtered_gyro: ResMut<FilteredGyroscope>, imu: Res<IMUValues>) {
    if imu.has_new_gyroscope_measurement() {
        filtered_gyro.update(imu.gyroscope);
    }
}

#[derive(Debug, Resource, Deref, DerefMut)]
pub struct BalanceAdjustment(f32);

fn update_balance_adjustment(
    mut balance_adjustment: ResMut<BalanceAdjustment>,
    filtered_gyro: Res<FilteredGyroscope>,
    config: Res<WalkingEngineConfig>,
) {
    **balance_adjustment = filtered_gyro.state().y * config.balancing.filtered_gyro_y_multiplier;
}
