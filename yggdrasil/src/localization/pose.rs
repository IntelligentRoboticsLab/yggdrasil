use bevy::prelude::*;
use filter::{StateMatrix, StateTransform, StateVector, WeightVector};
use num::Complex;

use crate::core::config::layout::LayoutConfig;

use nalgebra::{
    vector, ComplexField, Isometry2, Isometry3, Point2, Point3, SVector, Translation3, UnitComplex,
    UnitQuaternion, Vector2,
};
use nidhogg::types::HeadJoints;

#[derive(Resource, Default, Debug, Clone, Copy)]
pub struct RobotPose {
    pub inner: Isometry2<f32>,
}

impl RobotPose {
    // Constant for camera height that we set anywhere get_lookat_absolute is called.
    // Set to zero if we are only looking at the ground, for example.
    pub const CAMERA_HEIGHT: f32 = 0.5;

    #[must_use]
    pub fn from_isometry(pose: Isometry2<f32>) -> Self {
        Self { inner: pose }
    }

    #[must_use]
    pub fn from_translation_and_rotation(translation: Vector2<f32>, angle: f32) -> Self {
        let inner = Isometry2::new(translation, angle);
        Self { inner }
    }

    /// The current pose of the robot in the world, in 3D space.
    ///
    /// The z-axis is always 0.
    /// The rotation is around the z-axis.
    #[must_use]
    pub fn to_3d(&self) -> Isometry3<f32> {
        Isometry3::from_parts(
            Translation3::new(self.inner.translation.x, self.inner.translation.y, 0.0),
            UnitQuaternion::from_euler_angles(0.0, 0.0, self.inner.rotation.angle()),
        )
    }

    /// The current position of the robot in the world, in absolute coordinates.
    ///
    /// The center of the world is at the center of the field, with the x-axis pointing towards the
    /// opponent's goal.
    #[must_use]
    pub fn world_position(&self) -> Point2<f32> {
        self.inner.translation.vector.into()
    }

    /// The current rotation of the robot in the world, in radians.
    #[must_use]
    pub fn world_rotation(&self) -> f32 {
        self.inner.rotation.angle()
    }

    /// Transform a point from robot coordinates to world coordinates.
    #[must_use]
    pub fn robot_to_world(&self, point: &Point2<f32>) -> Point2<f32> {
        self.inner.transform_point(point)
    }

    /// Transform a point from world coordinates to robot coordinates.
    #[must_use]
    pub fn world_to_robot(&self, point: &Point2<f32>) -> Point2<f32> {
        self.inner.inverse_transform_point(point)
    }

    #[must_use]
    pub fn get_look_at_absolute(&self, point_in_world: &Point3<f32>) -> HeadJoints<f32> {
        let robot_to_point = self.world_to_robot(&point_in_world.xy());
        let x = robot_to_point.x;
        let y = robot_to_point.y;
        let z = point_in_world.z;
        let yaw = (robot_to_point.y / robot_to_point.x).atan();
        // 0.5 is the height of the robot's primary camera while standing
        let pitch = (0.5 - z).atan2((x * x + y * y).sqrt());

        HeadJoints { yaw, pitch }
    }

    #[must_use]
    pub fn distance_to(&self, point: &Point2<f32>) -> f32 {
        (self.world_position() - point).norm()
    }

    #[must_use]
    pub fn angle_to(&self, point: &Point2<f32>) -> f32 {
        let robot_to_point = self.world_to_robot(point).xy();
        robot_to_point.y.atan2(robot_to_point.x)
    }
}

impl From<RobotPose> for StateVector<3> {
    fn from(pose: RobotPose) -> Self {
        let translation = pose.inner.translation.vector;
        let rotation = pose.inner.rotation;
        translation.xy().push(rotation.angle())
    }
}

impl From<StateVector<3>> for RobotPose {
    fn from(state: StateVector<3>) -> Self {
        Self {
            inner: Isometry2::new(state.xy(), state.z),
        }
    }
}

impl StateTransform<3> for RobotPose {
    fn into_state_mean<const N: usize>(
        weights: WeightVector<N>,
        states: StateMatrix<3, N>,
    ) -> StateVector<3> {
        let mut mean_translation = SVector::zeros();
        let mut mean_angle = Complex::ZERO;

        for (&weight, pose) in weights.iter().zip(states.column_iter()) {
            mean_translation += weight * pose.xy();
            mean_angle += weight * Complex::cis(pose.z);
        }

        mean_translation.xy().push(mean_angle.argument())
    }

    fn residual(measurement: StateVector<3>, prediction: StateVector<3>) -> StateVector<3> {
        (measurement.xy() - prediction.xy())
            .push((UnitComplex::new(measurement.z) / UnitComplex::new(prediction.z)).angle())
    }
}

/// Returns the starting pose of the robot.
#[must_use]
pub fn initial_pose(layout: &LayoutConfig, player_num: u8) -> RobotPose {
    RobotPose::from_isometry(layout.initial_positions.player(player_num).isometry)
}

/// Returns the pose of the robot when it is penalized.
#[must_use]
pub fn penalized_pose(layout: &LayoutConfig) -> impl IntoIterator<Item = RobotPose> {
    /// "The removed robot will be placed outside the field at a distance of approximately 50 cm
    /// away from the nearest touchline, facing towards the field of play."
    const PENALTY_DISTANCE_FROM_TOUCHLINE: f32 = 0.5;

    [
        RobotPose::from_translation_and_rotation(
            vector![
                -layout.field.length / 2.0 + layout.field.penalty_mark_distance,
                -layout.field.width / 2.0 - PENALTY_DISTANCE_FROM_TOUCHLINE,
            ],
            std::f32::consts::FRAC_PI_2,
        ),
        RobotPose::from_translation_and_rotation(
            vector![
                -layout.field.length / 2.0 + layout.field.penalty_mark_distance,
                layout.field.width / 2.0 + PENALTY_DISTANCE_FROM_TOUCHLINE
            ],
            -std::f32::consts::FRAC_PI_2,
        ),
    ]
}

/// Returns the pose of the robot when taking a penalty kick.
#[must_use]
pub fn penalty_kick_pose(layout: &LayoutConfig, is_kicking_team: bool) -> RobotPose {
    if is_kicking_team {
        RobotPose::from_translation_and_rotation(
            vector![
                layout.field.length / 2.0 - layout.field.penalty_area_length,
                0.0
            ],
            0.0,
        )
    } else {
        RobotPose::from_translation_and_rotation(
            vector![layout.field.length / 2.0, 0.0],
            std::f32::consts::PI,
        )
    }
}
