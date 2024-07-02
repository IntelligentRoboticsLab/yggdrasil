use std::{f32::consts::PI, marker::PhantomData};

use crate::prelude::*;
use nalgebra::{Isometry3, Rotation3, Translation3, Vector3};
use nidhogg::NaoState;

use self::robot_dimensions::{ROBOT_TO_LEFT_PELVIS, ROBOT_TO_RIGHT_PELVIS};

pub mod forward;
pub mod inverse;
pub mod robot_dimensions;

pub use forward::RobotKinematics;

/// The kinematics module contains the kinematics of the robot.
///
/// The kinematics are updated using the joint angles read from the robot.
///
/// This module adds the following resources:
/// - [`RobotKinematics`]
pub struct KinematicsModule;

impl Module for KinematicsModule {
    fn initialize(self, app: App) -> Result<App> {
        Ok(app
            .init_resource::<RobotKinematics>()?
            .add_staged_system(SystemStage::Init, update_kinematics))
    }
}

#[system]
pub fn update_kinematics(robot_kinematics: &mut RobotKinematics, state: &NaoState) -> Result<()> {
    *robot_kinematics = RobotKinematics::from(&state.position);
    Ok(())
}

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
    pub fn zero(hip_height: f32) -> Self {
        Self {
            forward: 0.0,
            left: 0.0,
            turn: 0.0,
            hip_height,
            lift: 0.0,
        }
    }

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

/// The position of a specific foot relative to the robot's pelvis.
///
/// See [`FootOffset`] for more information.
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

/// Trait for kinematics of a specific foot of the robot.
///
/// This trait is used to implement the kinematics of the left and right foot of the robot.
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

        // Compute the foot roll in the pelvis frame
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
