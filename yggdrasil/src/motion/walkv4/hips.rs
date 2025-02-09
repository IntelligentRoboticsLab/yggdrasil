use bevy::prelude::*;

use crate::{
    kinematics::{self, Kinematics},
    nao::CycleTime,
};

use super::schedule::{MotionSet, StepPlanning};

/// Threshold for snapping to the requested hip height.
const REACHED_REQUESTED_THRESHOLD: f32 = 0.001;
/// Smoothing parameter for the exponential interpolation applied to the hip height.
const HEIGHT_ADJUSTMENT_SMOOTHING: f32 = 0.1;

pub(super) struct HipHeightPlugin;

impl Plugin for HipHeightPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(HipHeight {
            current: 0f32,
            requested: 0f32,
        });
        app.add_systems(PostStartup, init_hip_height.in_set(MotionSet::StepPlanning));
        app.add_systems(
            StepPlanning,
            update_hip_height.in_set(MotionSet::StepPlanning),
        );
    }
}

/// Resource containing the current and requested hip height, in meters from the ground.
#[derive(Debug, Clone, Resource)]
pub struct HipHeight {
    /// The current target hip height.
    /// This value is used when computing the joint angles for the legs.
    current: f32,
    /// The current requested hip height.
    /// This value is separate from the current hip height, to allow smooth
    /// interpolation between the two.
    requested: f32,
}

impl HipHeight {
    /// Get whether the hip height is currently being adjusted to the requested position.
    #[inline]
    #[must_use]
    pub fn is_adjusting(&self) -> bool {
        (self.current - self.requested).abs() > f32::EPSILON
    }

    /// Get the current target hip height.
    ///
    /// # Important
    ///
    /// This is not the physical hip height, but the hip height used when computing
    /// the joint angles for the legs of the robot.
    ///
    /// To obtain the physical hip height, use [`Kinematics`].
    #[must_use]
    pub fn current(&self) -> f32 {
        self.current + kinematics::dimensions::ANKLE_TO_SOLE.z
    }

    /// Get the requested hip height.
    ///
    /// # Important
    ///
    /// This is not the physical hip height, but the target hip height that is being interpolated towards.
    #[must_use]
    pub fn requested(&self) -> f32 {
        self.requested + kinematics::dimensions::ANKLE_TO_SOLE.z
    }

    /// Request a specific hip height.
    ///
    /// This will be propagated automatically by the [`HipHeightPlugin`].
    pub fn request(&mut self, hip_height: f32) {
        self.requested = hip_height;
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
        let step = HEIGHT_ADJUSTMENT_SMOOTHING * cycle_time.duration.as_secs_f32();
        let delta = difference.clamp(-step, step);
        hip_height.current += delta;
    }
}
