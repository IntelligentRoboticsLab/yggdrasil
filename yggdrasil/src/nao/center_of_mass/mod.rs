//! Center of mass (`CoM`) module.
//!
//! This module calculates the center of mass of the robot, and stores it in the [`CenterOfMass`]
//! resource. The `CoM` is calculated by taking the kinematic chain from the torso to each body part,
//! multiplying the mass of the body part by the position of the `CoM` of the body part, and summing
//! the results. The total mass of the robot is then divided out to get the `CoM`.
mod robot_masses;

use crate::{
    core::debug::DebugContext, kinematics::{spaces::Robot, Kinematics}, localization::RobotPose, prelude::*,
};
use bevy::prelude::*;
use nalgebra as na;
use spatial::types::Point3;
pub use robot_masses::*;

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

fn update_com(kinematics: Res<Kinematics>, mut com: ResMut<CenterOfMass>) {
    let new_com: na::Vector3<f32> = kinematics.to_robot(&TORSO.center).inner * TORSO.mass
        + kinematics.to_robot(&NECK.center).inner * NECK.mass
        + kinematics.to_robot(&HEAD.center).inner * HEAD.mass
        + kinematics.to_robot(&LEFT_SHOULDER.center).inner * LEFT_SHOULDER.mass
        + kinematics.to_robot(&LEFT_UPPER_ARM.center).inner * LEFT_UPPER_ARM.mass
        + kinematics.to_robot(&LEFT_ELBOW.center).inner * LEFT_ELBOW.mass
        + kinematics.to_robot(&LEFT_FOREARM.center).inner * LEFT_FOREARM.mass
        + kinematics.to_robot(&LEFT_WRIST.center).inner * LEFT_WRIST.mass
        + kinematics.to_robot(&RIGHT_SHOULDER.center).inner * RIGHT_SHOULDER.mass
        + kinematics.to_robot(&RIGHT_UPPER_ARM.center).inner * RIGHT_UPPER_ARM.mass
        + kinematics.to_robot(&RIGHT_ELBOW.center).inner * RIGHT_ELBOW.mass
        + kinematics.to_robot(&RIGHT_FOREARM.center).inner * RIGHT_FOREARM.mass
        + kinematics.to_robot(&RIGHT_WRIST.center).inner * RIGHT_WRIST.mass
        + kinematics.to_robot(&LEFT_HIP.center).inner * LEFT_HIP.mass
        + kinematics.to_robot(&LEFT_PELVIS.center).inner * LEFT_PELVIS.mass
        + kinematics.to_robot(&LEFT_THIGH.center).inner * LEFT_THIGH.mass
        + kinematics.to_robot(&LEFT_TIBIA.center).inner * LEFT_TIBIA.mass
        + kinematics.to_robot(&LEFT_ANKLE.center).inner * LEFT_ANKLE.mass
        + kinematics.to_robot(&LEFT_FOOT.center).inner * LEFT_FOOT.mass
        + kinematics.to_robot(&RIGHT_HIP.center).inner * RIGHT_HIP.mass
        + kinematics.to_robot(&RIGHT_PELVIS.center).inner * RIGHT_PELVIS.mass
        + kinematics.to_robot(&RIGHT_THIGH.center).inner * RIGHT_THIGH.mass
        + kinematics.to_robot(&RIGHT_TIBIA.center).inner * RIGHT_TIBIA.mass
        + kinematics.to_robot(&RIGHT_ANKLE.center).inner * RIGHT_ANKLE.mass;

    *com = CenterOfMass {
        position: na::Point3::from(new_com / TOTAL_MASS).into(),
    };
}

fn setup_com_visualization(dbg: DebugContext) {
    dbg.log_component_batches(
        "localization/pose/com",
        true,
        [&rerun::Color::from_rgb(255, 64, 0) as _],
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
