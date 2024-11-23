use bevy::prelude::*;
use nalgebra as na;
use crate::{core::debug::DebugContext, localization::RobotPose};

use spatial::types::Isometry3;

use super::prelude::*;

macro_rules! log_meshes {
    {$($name:ident: $space:ty,)*} => {
        pub fn setup_meshes(dbg: DebugContext) {
            $(
                let path = concat!("nao/", stringify!($name));

                dbg.log_static(
                    path,
                    &rerun::Asset3D::from_file(concat!("./assets/rerun/nao/", stringify!($name), ".glb"))
                        .expect(concat!("Failed to load ", stringify!($), " model"))
                        .with_media_type(rerun::MediaType::glb()),
                );

                dbg.log_static(path, &rerun::ViewCoordinates::FLU);
            )*
        }

        pub fn update_meshes(dbg: DebugContext, kinematics: Res<Kinematics>, pose: Res<RobotPose>) {
            let height = kinematics
                .vector::<LeftSole, Robot>()
                .inner
                .z
                .max(kinematics.vector::<RightSole, Robot>().inner.z);

            let robot_to_field: Isometry3<Robot, Field> = pose.as_3d().into();
            let robot_to_field = robot_to_field.map(|x| x * na::Translation3::new(0., 0., height));

            $(
                let isometry = kinematics.isometry::<$space, Robot>().chain(robot_to_field.as_ref());

                dbg.log(
                    concat!("nao/", stringify!($name)),
                    &rerun::Transform3D::from_translation_rotation(
                        isometry.inner.translation.vector.data.0[0],
                        rerun::Quaternion(isometry.inner.rotation.coords.data.0[0]),
                    )
                );
            )*
        }
    };
}

log_meshes! {
   	head: Head,
	left_ankle: LeftAnkle,
	left_foot: LeftFoot,
	left_forearm: LeftForearm,
	left_hip: LeftHip,
	left_pelvis: LeftPelvis,
	left_shoulder: LeftShoulder,
	left_thigh: LeftThigh,
	left_tibia: LeftTibia,
	left_upper_arm: LeftUpperArm,
	left_wrist: LeftWrist,
	neck: Neck,
	right_ankle: RightAnkle,
	right_foot: RightFoot,
	right_forearm: RightForearm,
	right_hip: RightHip,
	right_pelvis: RightPelvis,
	right_shoulder: RightShoulder,
	right_thigh: RightThigh,
	right_tibia: RightTibia,
	right_upper_arm: RightUpperArm,
	right_wrist: RightWrist,
	robot: Robot,
}
