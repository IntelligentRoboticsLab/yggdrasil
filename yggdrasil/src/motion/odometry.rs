use bevy::prelude::*;
use nalgebra::{Isometry2, Translation2, UnitComplex, Vector2};

use serde::{Deserialize, Serialize};

use crate::{
    behavior::{behaviors::Standup, engine::in_behavior},
    kinematics::{
        spaces::{LeftSole, RightSole},
        Kinematics,
    },
    sensor::{falling::FallState, orientation::RobotOrientation},
};

use super::walking_engine::{foot_support::FootSupportState, Side, WalkingEngineSet};

/// Plugin that keeps track of the odometry of the robot.
pub(super) struct OdometryPlugin;

impl Plugin for OdometryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Odometry>().add_systems(
            PreUpdate,
            update_odometry
                .run_if(not(in_behavior::<Standup>))
                .after(crate::kinematics::update_kinematics)
                .after(crate::sensor::orientation::update_orientation)
                .after(WalkingEngineSet::Prepare),
        );
    }
}

/// System that updates the robot odometry, given the current state of the robot joints.
pub fn update_odometry(
    mut odometry: ResMut<Odometry>,
    odometry_config: Res<OdometryConfig>,
    foot_support: Res<FootSupportState>,
    kinematics: Res<Kinematics>,
    orientation: Res<RobotOrientation>,
    fall_state: Res<FallState>,
) {
    if !matches!(*fall_state, FallState::None) {
        // Don't update odometry if the robot is falling, or getting up
        odometry.offset_to_last = Isometry2::default();
        return;
    }

    // TODO: We should probably reset the odometry in some cases
    // See: https://github.com/IntelligentRoboticsLab/yggdrasil/issues/400
    odometry.update(&odometry_config, &foot_support, &kinematics, &orientation);
}

/// Configuration for the odometry.
#[derive(Resource, Debug, Clone, Serialize, Deserialize)]
pub struct OdometryConfig {
    /// The scale factor to apply to the odometry.
    pub scale_factor: Vector2<f32>,
}

/// The odometry of the robot.
#[derive(Resource, Debug, Default, Clone)]
pub struct Odometry {
    /// The accumulated odometry offset of the robot.
    pub accumulated: Isometry2<f32>,
    /// The offset to the last position of the robot.
    pub offset_to_last: Isometry2<f32>,
    last_left_sole_to_right_sole: Vector2<f32>,
    last_orientation: UnitComplex<f32>,
}

impl Odometry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset the orientation of the robot to the given [`RobotOrientation`].
    pub fn reset_orientation(&mut self, orientation: &RobotOrientation) {
        self.last_orientation = UnitComplex::from_angle(orientation.euler_angles().2);
        self.accumulated.rotation = self.last_orientation;
        self.offset_to_last.rotation = UnitComplex::identity();
    }

    /// Update the odometry of the robot using the given [`Kinematics`].
    pub fn update(
        &mut self,

        config: &OdometryConfig,
        foot_support: &FootSupportState,
        kinematics: &Kinematics,
        orientation: &RobotOrientation,
    ) {
        let left_sole_to_right_sole = kinematics.vector::<LeftSole, RightSole>().inner.xy();

        // Compute offset to last position, divided by 2 to get the center of the robot.
        let offset = match foot_support.support_side() {
            Side::Left => left_sole_to_right_sole - self.last_left_sole_to_right_sole,
            Side::Right => -left_sole_to_right_sole + self.last_left_sole_to_right_sole,
        } / 2.0;

        self.last_left_sole_to_right_sole = left_sole_to_right_sole;
        let scaled_offset = offset.component_mul(&config.scale_factor);

        let yaw = UnitComplex::from_angle(orientation.euler_angles().2);
        let orientation_offset = self.last_orientation.rotation_to(&yaw);
        self.last_orientation = yaw;

        let odometry_offset =
            Isometry2::from_parts(Translation2::from(scaled_offset), orientation_offset);

        // update the accumulated odometry
        self.offset_to_last = odometry_offset;
        self.accumulated *= odometry_offset;
    }
}
