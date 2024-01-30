use std::{f32::consts::PI, marker::PhantomData};

use nalgebra::{Isometry3, Rotation3, Translation3, Vector3};

use self::robot_dimensions::{ROBOT_TO_LEFT_PELVIS, ROBOT_TO_RIGHT_PELVIS};

pub mod inverse;

/// The position of a foot relative to the robot's torso.
///
/// The origin is the center of the robot's torso, the x-axis points forward, the y-axis points left,
/// and the z-axis points up.
///
/// ## Units
/// - `forward`: meters
/// - `left`: meters
/// - `turn`: radians
/// - `hip_height`: meters
/// - `lift`: meters
/// - `side`: `Left` or `Right`
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct FootOffset {
    pub forward: f32,
    pub left: f32,
    pub turn: f32,
    pub hip_height: f32,
    pub lift: f32,
}

impl FootOffset {
    pub(crate) fn into_left(self) -> SidedFootOffset<Left> {
        SidedFootOffset {
            forward: self.forward,
            left: self.left,
            turn: self.turn,
            hip_height: self.hip_height,
            lift: self.lift,
            _side: PhantomData,
        }
    }

    pub(crate) fn into_right(self) -> SidedFootOffset<Right> {
        SidedFootOffset {
            forward: self.forward,
            left: self.left,
            turn: self.turn,
            hip_height: self.hip_height,
            lift: self.lift,
            _side: PhantomData,
        }
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct SidedFootOffset<T: FootKinematics> {
    pub forward: f32,
    pub left: f32,
    pub turn: f32,
    pub hip_height: f32,
    pub lift: f32,
    _side: PhantomData<T>,
}

pub(crate) struct Left;
pub(crate) struct Right;

pub trait FootKinematics {
    fn torso_to_pelvis() -> Isometry3<f32>;
    fn robot_to_pelvis() -> Isometry3<f32>;
    fn foot_rotation(turn: f32) -> Isometry3<f32>;
}

impl FootKinematics for Left {
    fn torso_to_pelvis() -> Isometry3<f32> {
        Isometry3::rotation(Vector3::x() * PI / -4.0) * Translation3::from(-ROBOT_TO_LEFT_PELVIS)
    }

    fn robot_to_pelvis() -> Isometry3<f32> {
        Isometry3::from(ROBOT_TO_LEFT_PELVIS)
    }

    fn foot_rotation(turn: f32) -> Isometry3<f32> {
        Isometry3::rotation(Vector3::z() * turn)
    }
}

impl FootKinematics for Right {
    fn torso_to_pelvis() -> Isometry3<f32> {
        Isometry3::rotation(Vector3::x() * PI / 4.0) * Translation3::from(-ROBOT_TO_RIGHT_PELVIS)
    }
    fn robot_to_pelvis() -> Isometry3<f32> {
        Isometry3::from(ROBOT_TO_RIGHT_PELVIS)
    }

    fn foot_rotation(turn: f32) -> Isometry3<f32> {
        Isometry3::rotation(Vector3::z() * -turn)
    }
}

impl<Side> SidedFootOffset<Side>
where
    Side: FootKinematics,
{
    /// Compute the transformation from this [`FootOffset`] to the robot's torso.
    ///
    /// The [`FootOffset`] is relative to the robot's torso, and the transformation is relative to the
    /// robot's torso.
    #[inline]
    fn to_torso(&self, torso_offset: f32) -> Isometry3<f32> {
        let SidedFootOffset {
            forward,
            left,
            turn,
            hip_height,
            lift,
            _side,
        } = self;
        let foot_translation =
            Isometry3::translation(forward - torso_offset, *left, -hip_height + lift);
        let rotation = Side::foot_rotation(*turn);

        Side::robot_to_pelvis() * foot_translation * rotation
    }

    /// Compute the transformation from this [`FootOffset`] to the robot's pelvis.
    ///
    /// The [`FootOffset`] is relative to the robot's torso, and the transformation is relative to the
    /// robot's pelvis.
    #[inline]
    fn to_pelvis(&self, torso_offset: f32) -> Isometry3<f32> {
        Side::torso_to_pelvis() * self.to_torso(torso_offset)
    }

    #[inline]
    fn compute_hip_yaw_pitch(foot_to_pelvis: &Isometry3<f32>) -> f32 {
        // get vector pointing from pelvis to foot, to compute the angles
        let pelvis_to_foot = foot_to_pelvis.inverse().translation;

        // TODO: diagram
        let foot_roll_in_pelvis = pelvis_to_foot.y.atan2(pelvis_to_foot.z);

        // Compute the foot pitch in the pelvis frame, by projecting the foot vector
        let foot_pitch_in_pelvis = pelvis_to_foot
            .x
            .atan2((pelvis_to_foot.y.powi(2) + pelvis_to_foot.z.powi(2)).sqrt());

        let rotation = Rotation3::new(Vector3::x() * -1.0 * foot_roll_in_pelvis)
            * Rotation3::new(Vector3::y() * foot_pitch_in_pelvis);

        // foot_to_pelvis contains z component, we apply the y component using the rotation computed earlier
        let hip_rotation_c1 = foot_to_pelvis.rotation * (rotation * Vector3::y());

        (-1.0 * hip_rotation_c1.x).atan2(hip_rotation_c1.y)
    }
}

/// The robot's dimensions, in meters.
///
/// The origin is the center of the robot's torso, the x-axis points forward, the y-axis points left,
/// and the z-axis points up.
pub mod robot_dimensions {
    use nalgebra::{vector, Vector3};
    /// Vector pointing from torso to robot frame.
    pub const TORSO_TO_ROBOT: Vector3<f32> = vector![-0.0413, 0.0, -0.12842];
    /// Vector pointing from robot frame to neck.
    pub const ROBOT_TO_NECK: Vector3<f32> = vector![0.0, 0.0, 0.2115];
    /// Vector pointing from robot frame to the left pelvis.
    pub const ROBOT_TO_LEFT_PELVIS: Vector3<f32> = vector![0.0, 0.05, 0.0];
    /// Vector pointing from robot frame to the right pelvis.
    pub const ROBOT_TO_RIGHT_PELVIS: Vector3<f32> = vector![0.0, -0.05, 0.0];
    /// Vector pointing from the hip to the knee (identical for both legs).
    pub const HIP_TO_KNEE: Vector3<f32> = vector![0.0, 0.0, -0.1];
    /// Vector pointing from the knee to the ankle (identical for both legs).
    pub const KNEE_TO_ANKLE: Vector3<f32> = vector![0.0, 0.0, -0.1029];
    /// Vector pointing from the ankle to the sole (identical for both legs).
    pub const ANKLE_TO_SOLE: Vector3<f32> = vector![0.0, 0.0, -0.04519];
    /// Vector pointing from the robot frame to the left shoulder.
    pub const ROBOT_TO_LEFT_SHOULDER: Vector3<f32> = vector![0.0, 0.098, 0.185];
    /// Vector pointing from the robot frame to the right shoulder.
    pub const ROBOT_TO_RIGHT_SHOULDER: Vector3<f32> = vector![0.0, -0.098, 0.185];
    /// Vector pointing from the left shoulder to the left elbow.
    pub const LEFT_SHOULDER_TO_LEFT_ELBOW: Vector3<f32> = vector![0.105, 0.015, 0.0];
    /// Vector pointing from the right shoulder to the right elbow.
    pub const RIGHT_SHOULDER_TO_RIGHT_ELBOW: Vector3<f32> = vector![0.105, -0.015, 0.0];
    /// Vector pointing from the elbow to the wrist (identical for both arms).
    pub const ELBOW_TO_WRIST: Vector3<f32> = vector![0.05595, 0.0, 0.0];
    /// Vector pointing from the neck to the top camera.
    pub const NECK_TO_TOP_CAMERA: Vector3<f32> = vector![0.05871, 0.0, 0.06364];
    /// Vector pointing from the neck to the bottom camera.
    pub const NECK_TO_BOTTOM_CAMERA: Vector3<f32> = vector![0.05071, 0.0, 0.01774];
}
