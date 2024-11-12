use bevy::prelude::*;
use nalgebra::{Isometry2, Translation2, UnitComplex, Vector2};

use serde::{Deserialize, Serialize};

use crate::{
    behavior::primary_state::PrimaryState,
    kinematics::{
        spaces::{LeftSole, RightSole},
        Kinematics,
    },
    motion::walk::{engine::Side, SwingFoot},
    sensor::orientation::RobotOrientation,
};

/// Plugin that keeps track of the odometry of the robot.
pub(super) struct OdometryPlugin;

impl Plugin for OdometryPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Odometry>().add_systems(
            PreUpdate,
            update_odometry
                .after(crate::kinematics::update_kinematics)
                .after(crate::sensor::orientation::update_orientation),
        );
    }
}

/// System that updates the robot odometry, given the current state of the robot joints.
pub fn update_odometry(
    mut odometry: ResMut<Odometry>,
    odometry_config: Res<OdometryConfig>,
    swing_foot: Res<SwingFoot>,
    kinematics: Res<Kinematics>,
    orientation: Res<RobotOrientation>,
    primary_state: Res<PrimaryState>,
) {
    match *primary_state {
        PrimaryState::Penalized | PrimaryState::Initial | PrimaryState::Sitting => {
            *odometry = Odometry::default();
        }
        _ => {
            odometry.update(&odometry_config, &swing_foot, &kinematics, &orientation);
        }
    }
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

    /// Update the odometry of the robot using the given [`Kinematics`].
    pub fn update(
        &mut self,
        config: &OdometryConfig,
        swing_foot: &SwingFoot,
        kinematics: &Kinematics,
        orientation: &RobotOrientation,
    ) {
        let left_sole_to_right_sole = kinematics
            .isometry::<RightSole, LeftSole>()
            .inner
            .translation
            .vector
            .xy();

        // Compute offset to last position, divided by 2 to get the center of the robot.
        let offset = match swing_foot.support() {
            Side::Left => left_sole_to_right_sole - self.last_left_sole_to_right_sole,
            Side::Right => -left_sole_to_right_sole + self.last_left_sole_to_right_sole,
        } / 2.0;

        self.last_left_sole_to_right_sole = left_sole_to_right_sole;
        let scaled_offset = offset.component_mul(&config.scale_factor);

        let yaw = UnitComplex::from_angle(orientation.euler().2);
        self.last_orientation = yaw;

        let odometry_offset =
            Isometry2::from_parts(Translation2::from(scaled_offset), UnitComplex::identity());

        let raw = Isometry2::from_parts(self.accumulated.translation, UnitComplex::identity());

        // update the accumulated odometry
        self.offset_to_last = odometry_offset;
        self.accumulated = Isometry2::from_parts((raw * odometry_offset).translation, yaw);
    }
}
