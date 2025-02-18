//! Center of mass (`CoM`) module.
//!
//! This module calculates the center of mass of the robot, and stores it in the [`CenterOfMass`]
//! resource. The `CoM` is calculated by taking the kinematic chain from the torso to each body part,
//! multiplying the mass of the body part by the position of the `CoM` of the body part, and summing
//! the results. The total mass of the robot is then divided out to get the `CoM`.

mod robot_masses;

use crate::{
    core::debug::DebugContext,
    kinematics::{spaces::Robot, Kinematics},
    localization::RobotPose,
    prelude::*,
};
use bevy::prelude::*;
pub use robot_masses::*;
use spatial::types::Point3;

/// Plugin which adds the `CoM` of the robot to the storage, and updates it each cycle.
///
/// Adds the following resources:
/// - [`CenterOfMass`] - The center of mass of the robot.
pub struct CenterOfMassPlugin;

impl Plugin for CenterOfMassPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CenterOfMass>();
        app.add_systems(PostStartup, setup_com_visualization)
            .add_systems(
                Sensor,
                update_com.after(crate::kinematics::update_kinematics),
            )
            .add_systems(PostUpdate, visualize_com);
    }
}

/// The center of mass of the robot.
///
/// This is updated each cycle, by taking the kinematic chain from the torso to each body part,
/// multiplying the mass of the body part by the position of the center of mass of the body part,
/// and summing the results. The total mass of the robot is then divided out to get the center of
/// mass.
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq)]
pub struct CenterOfMass {
    /// The center of mass of the robot in *robot* frame.
    pub position: Point3<Robot>,
}

pub(super) fn update_com(kinematics: Res<Kinematics>, mut com: ResMut<CenterOfMass>) {
    let new_com = kinematics.transform(&TORSO.center) * TORSO.mass
        + kinematics.transform(&NECK.center) * NECK.mass
        + kinematics.transform(&HEAD.center) * HEAD.mass
        + kinematics.transform(&LEFT_SHOULDER.center) * LEFT_SHOULDER.mass
        + kinematics.transform(&LEFT_UPPER_ARM.center) * LEFT_UPPER_ARM.mass
        + kinematics.transform(&LEFT_ELBOW.center) * LEFT_ELBOW.mass
        + kinematics.transform(&LEFT_FOREARM.center) * LEFT_FOREARM.mass
        + kinematics.transform(&LEFT_WRIST.center) * LEFT_WRIST.mass
        + kinematics.transform(&RIGHT_SHOULDER.center) * RIGHT_SHOULDER.mass
        + kinematics.transform(&RIGHT_UPPER_ARM.center) * RIGHT_UPPER_ARM.mass
        + kinematics.transform(&RIGHT_ELBOW.center) * RIGHT_ELBOW.mass
        + kinematics.transform(&RIGHT_FOREARM.center) * RIGHT_FOREARM.mass
        + kinematics.transform(&RIGHT_WRIST.center) * RIGHT_WRIST.mass
        + kinematics.transform(&LEFT_HIP.center) * LEFT_HIP.mass
        + kinematics.transform(&LEFT_PELVIS.center) * LEFT_PELVIS.mass
        + kinematics.transform(&LEFT_THIGH.center) * LEFT_THIGH.mass
        + kinematics.transform(&LEFT_TIBIA.center) * LEFT_TIBIA.mass
        + kinematics.transform(&LEFT_ANKLE.center) * LEFT_ANKLE.mass
        + kinematics.transform(&LEFT_FOOT.center) * LEFT_FOOT.mass
        + kinematics.transform(&RIGHT_HIP.center) * RIGHT_HIP.mass
        + kinematics.transform(&RIGHT_PELVIS.center) * RIGHT_PELVIS.mass
        + kinematics.transform(&RIGHT_THIGH.center) * RIGHT_THIGH.mass
        + kinematics.transform(&RIGHT_TIBIA.center) * RIGHT_TIBIA.mass
        + kinematics.transform(&RIGHT_ANKLE.center) * RIGHT_ANKLE.mass;

    *com = CenterOfMass {
        position: (new_com / TOTAL_MASS).map(From::from),
    };
}

fn setup_com_visualization(dbg: DebugContext) {
    dbg.log_static(
        "localization/pose/com",
        &rerun::Points3D::update_fields().with_colors([(255, 64, 0)]),
    );
}

fn visualize_com(dbg: DebugContext, com: Res<CenterOfMass>, pose: Res<RobotPose>) {
    let absolute_com_position = pose.robot_to_world(&com.position.inner.xy());
    dbg.log(
        "localization/pose/com",
        &rerun::Points3D::new([(
            absolute_com_position.x,
            absolute_com_position.y,
            com.position.inner.z,
        )])
        .with_radii([0.005]),
    );
}
