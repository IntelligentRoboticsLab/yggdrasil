use bevy::prelude::*;
use nalgebra as na;
use std::{f32::consts::PI, marker::PhantomData};

use nidhogg::NaoState;

use self::dimensions::{ROBOT_TO_LEFT_PELVIS, ROBOT_TO_RIGHT_PELVIS};
use self::spaces::{Left, Right};
use self::visualization::KinematicsVisualizationPlugin;

pub mod dimensions;
pub mod forward;
pub mod inverse;
pub mod spaces;
pub mod visualization;

pub mod prelude {
    pub use super::Kinematics;
    pub use super::dimensions::*;
    pub use super::spaces::*;
}

pub use forward::Kinematics;

/// Plugin for the kinematics of the robot.
///
/// The kinematics are updated using the joint angles read from the robot.
pub struct KinematicsPlugin;

impl Plugin for KinematicsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_kinematics)
            .add_plugins(KinematicsVisualizationPlugin)
            .add_systems(PreUpdate, update_kinematics);
    }
}

/// System that initializes the [`Kinematics`] struct based on the current state.
pub fn init_kinematics(mut commands: Commands, state: Res<NaoState>) {
    commands.insert_resource(Kinematics::from(&state.position));
}

/// System that updates the [`Kinematics`] resource.
pub fn update_kinematics(mut kinematics: ResMut<Kinematics>, state: Res<NaoState>) {
    *kinematics = Kinematics::from(&state.position);
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
    #[must_use]
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

/// Trait for kinematics of a specific foot of the robot.
///
/// This trait is used to implement the kinematics of the left and right foot of the robot.
pub trait FootKinematics {
    fn torso_to_pelvis() -> na::Isometry3<f32>;
    fn robot_to_pelvis() -> na::Isometry3<f32>;
    fn foot_rotation(turn: f32) -> na::Isometry3<f32>;
}

impl FootKinematics for Left {
    fn torso_to_pelvis() -> na::Isometry3<f32> {
        na::Isometry3::rotation(na::Vector3::x() * PI / -4.0)
            * na::Translation3::from(-ROBOT_TO_LEFT_PELVIS)
    }

    fn robot_to_pelvis() -> na::Isometry3<f32> {
        na::Isometry3::from(ROBOT_TO_LEFT_PELVIS)
    }

    fn foot_rotation(turn: f32) -> na::Isometry3<f32> {
        na::Isometry3::rotation(na::Vector3::z() * turn)
    }
}

impl FootKinematics for Right {
    fn torso_to_pelvis() -> na::Isometry3<f32> {
        na::Isometry3::rotation(na::Vector3::x() * PI / 4.0)
            * na::Translation3::from(-ROBOT_TO_RIGHT_PELVIS)
    }
    fn robot_to_pelvis() -> na::Isometry3<f32> {
        na::Isometry3::from(ROBOT_TO_RIGHT_PELVIS)
    }

    fn foot_rotation(turn: f32) -> na::Isometry3<f32> {
        na::Isometry3::rotation(na::Vector3::z() * -turn)
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
    fn to_torso(&self, torso_offset: f32) -> na::Isometry3<f32> {
        let SidedFootOffset {
            forward,
            left,
            turn,
            hip_height,
            lift,
            _side,
        } = self;
        let foot_translation = na::Isometry3::translation(
            forward - torso_offset,
            *left,
            -(hip_height + dimensions::ANKLE_TO_SOLE.z) + lift, // foot translation is computed from the ankle
        );
        let rotation = Side::foot_rotation(*turn);

        Side::robot_to_pelvis() * foot_translation * rotation
    }

    /// Compute the transformation from this [`FootOffset`] to the robot's pelvis.
    ///
    /// The [`FootOffset`] is relative to the robot's torso, and the transformation is relative to the
    /// robot's pelvis.
    #[inline]
    fn to_pelvis(&self, torso_offset: f32) -> na::Isometry3<f32> {
        Side::torso_to_pelvis() * self.to_torso(torso_offset)
    }

    #[inline]
    fn compute_hip_yaw_pitch(foot_to_pelvis: &na::Isometry3<f32>) -> f32 {
        // get vector pointing from pelvis to foot, to compute the angles
        let pelvis_to_foot = foot_to_pelvis.inverse().translation;

        // Compute the foot roll in the pelvis frame
        let foot_roll_in_pelvis = pelvis_to_foot.y.atan2(pelvis_to_foot.z);

        // Compute the foot pitch in the pelvis frame, by projecting the foot vector
        let foot_pitch_in_pelvis = pelvis_to_foot
            .x
            .atan2((pelvis_to_foot.y.powi(2) + pelvis_to_foot.z.powi(2)).sqrt());

        let rotation = na::Rotation3::new(na::Vector3::x() * -1.0 * foot_roll_in_pelvis)
            * na::Rotation3::new(na::Vector3::y() * foot_pitch_in_pelvis);

        // foot_to_pelvis contains z component, we apply the y component using the rotation computed earlier
        let hip_rotation_c1 = foot_to_pelvis.rotation * (rotation * na::Vector3::y());

        (-1.0 * hip_rotation_c1.x).atan2(hip_rotation_c1.y)
    }
}
