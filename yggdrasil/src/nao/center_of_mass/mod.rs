//! Center of mass (`CoM`) module.
//!
//! This module calculates the center of mass of the robot, and stores it in the [`CenterOfMass`]
//! resource. The `CoM` is calculated by taking the kinematic chain from the torso to each body part,
//! multiplying the mass of the body part by the position of the `CoM` of the body part, and summing
//! the results. The total mass of the robot is then divided out to get the `CoM`.
mod robot_masses;

use crate::{
    core::debug::DebugContext, kinematics::RobotKinematics, localization::RobotPose, prelude::*,
};
use bevy::prelude::*;
use nalgebra::Point3;
pub use robot_masses::*;

/// Plugin which adds the `CoM` of the robot to the storage, and updates it each cycle.
///
/// Adds the following resources:
/// - [`CenterOfMass`] - The center of mass of the robot.
pub struct CenterOfMassPlugin;

impl Plugin for CenterOfMassPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CenterOfMass>();
        app.add_systems(
            Sensor,
            (
                update_com.after(crate::kinematics::update_kinematics),
                log_com,
            )
                .chain(),
        );
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
    pub position: Point3<f32>,
}

fn update_com(kinematics: Res<RobotKinematics>, mut com: ResMut<CenterOfMass>) {
    let new_com = kinematics.torso_to_robot * TORSO.center * TORSO.mass
        + kinematics.neck_to_robot * NECK.center * NECK.mass
        + kinematics.head_to_robot * HEAD.center * HEAD.mass
        + kinematics.left_shoulder_to_robot * LEFT_SHOULDER.center * LEFT_SHOULDER.mass
        + kinematics.left_upper_arm_to_robot * LEFT_UPPER_ARM.center * LEFT_UPPER_ARM.mass
        + kinematics.left_elbow_to_robot * LEFT_ELBOW.center * LEFT_ELBOW.mass
        + kinematics.left_forearm_to_robot * LEFT_FOREARM.center * LEFT_FOREARM.mass
        + kinematics.left_wrist_to_robot * LEFT_WRIST.center * LEFT_WRIST.mass
        + kinematics.right_shoulder_to_robot * RIGHT_SHOULDER.center * RIGHT_SHOULDER.mass
        + kinematics.right_upper_arm_to_robot * RIGHT_UPPER_ARM.center * RIGHT_UPPER_ARM.mass
        + kinematics.right_elbow_to_robot * RIGHT_ELBOW.center * RIGHT_ELBOW.mass
        + kinematics.right_forearm_to_robot * RIGHT_FOREARM.center * RIGHT_FOREARM.mass
        + kinematics.right_wrist_to_robot * RIGHT_WRIST.center * RIGHT_WRIST.mass
        + kinematics.left_hip_to_robot * LEFT_HIP.center * LEFT_HIP.mass
        + kinematics.left_pelvis_to_robot * LEFT_PELVIS.center * LEFT_PELVIS.mass
        + kinematics.left_thigh_to_robot * LEFT_THIGH.center * LEFT_THIGH.mass
        + kinematics.left_tibia_to_robot * LEFT_TIBIA.center * LEFT_TIBIA.mass
        + kinematics.left_ankle_to_robot * LEFT_ANKLE.center * LEFT_ANKLE.mass
        + kinematics.left_foot_to_robot * LEFT_FOOT.center * LEFT_FOOT.mass
        + kinematics.right_hip_to_robot * RIGHT_HIP.center * RIGHT_HIP.mass
        + kinematics.right_pelvis_to_robot * RIGHT_PELVIS.center * RIGHT_PELVIS.mass
        + kinematics.right_thigh_to_robot * RIGHT_THIGH.center * RIGHT_THIGH.mass
        + kinematics.right_tibia_to_robot * RIGHT_TIBIA.center * RIGHT_TIBIA.mass
        + kinematics.right_ankle_to_robot * RIGHT_ANKLE.center * RIGHT_ANKLE.mass;

    *com = CenterOfMass {
        position: (new_com / TOTAL_MASS).into(),
    };
}

fn log_com(_dbg: DebugContext, _com: Res<CenterOfMass>, _pose: Res<RobotPose>) {
    // TODO: Visualize
    // let absolute_com_position = pose.robot_to_world(&com.position.xy());
    // dbg.log_points_3d_with_color_and_radius(
    //     "/localisation/pose/com",
    //     &[(
    //         absolute_com_position.x,
    //         absolute_com_position.y,
    //         com.position.z,
    //     )],
    //     nidhogg::types::color::u8::MAROON,
    //     0.005,
    // )
    // .expect("failed to log center of mass");
}
