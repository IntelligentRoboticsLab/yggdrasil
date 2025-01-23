use bevy::prelude::*;

use crate::{kinematics::Kinematics, motion::walk::WalkingEngineConfig, nao::CycleTime};

use super::scheduling::MotionSet;

/// Threshold for snapping to the requested hip height.
const REACHED_REQUESTED_THRESHOLD: f32 = 0.005;
/// Smoothing parameter for the exponential interpolation applied to the hip height.
const HEIGHT_ADJUSTMENT_SMOOTHING: f32 = 1.0;

pub(super) struct HipHeightPlugin;

impl Plugin for HipHeightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, init_hip_height);
        app.add_systems(Update, update_hip_height.in_set(MotionSet::StepPlanning));
    }
}

/// Resource containing the current and requested hip height, in meters from the ground.
#[derive(Debug, Clone, Resource)]
pub struct HipHeight {
    /// The current target hip height.
    /// This value is used when computing the joint angles for the legs.
    pub current: f32,
    /// The current requested hip height.
    /// This value is separate from the current hip height, to allow smooth
    /// interpolation between the two.
    pub requested: f32,
}

impl HipHeight {
    /// Get whether the hip height is currently being adjusted to the requested position.
    #[inline]
    #[must_use]
    fn is_adjusting(&self) -> bool {
        self.current != self.requested
    }
}

fn init_hip_height(mut commands: Commands, kinematics: Res<Kinematics>) {
    let hip_height = kinematics.left_hip_height();
    commands.insert_resource(HipHeight {
        current: hip_height,
        requested: hip_height,
    });
}

fn update_hip_height(mut hip_height: ResMut<HipHeight>, cycle_time: Res<CycleTime>) {
    let difference = hip_height.requested - hip_height.current;

    // If the difference is very small, snap to the target
    if difference.abs() < REACHED_REQUESTED_THRESHOLD {
        hip_height.current = hip_height.requested;
    } else {
        // Otherwise, perform exponential interpolation
        let t = 1.0 - (-HEIGHT_ADJUSTMENT_SMOOTHING * cycle_time.duration.as_secs_f32()).exp();
        hip_height.current += difference * t;
    }
}
