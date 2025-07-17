use bevy::prelude::*;
use rerun::Transform3D;

use super::prelude::*;
use crate::{
    core::debug::DebugContext, localization::RobotPose, nao::Cycle,
    sensor::orientation::RobotOrientation,
};

pub struct KinematicsVisualizationPlugin;

const RERUN_BATCH_SIZE: usize = 50;

macro_rules! meshes {
    ($($x:tt)*) => {
        macro_rules! x {$($x)*}
        x! {
            head, Head
            neck, Neck
            robot, Robot
            left_shoulder, LeftShoulder
            left_upper_arm, LeftUpperArm
            left_forearm, LeftForearm
            left_wrist, LeftWrist
            left_pelvis, LeftPelvis
            left_hip, LeftHip
            left_thigh, LeftThigh
            left_tibia, LeftTibia
            left_ankle, LeftAnkle
            left_foot, LeftFoot
            right_shoulder, RightShoulder
            right_upper_arm, RightUpperArm
            right_forearm, RightForearm
            right_wrist, RightWrist
            right_pelvis, RightPelvis
            right_hip, RightHip
            right_thigh, RightThigh
            right_tibia, RightTibia
            right_ankle, RightAnkle
            right_foot, RightFoot
        }
    };
}

impl Plugin for KinematicsVisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostStartup, setup_meshes)
            .add_systems(PostUpdate, update_meshes);
    }
}

meshes! {($($name:ident, $_:ty)*) => {
    #[derive(Default)]
    struct TransformBuffer {
        cycle: Vec<i64>,
        $($name: (Vec<rerun::components::Translation3D>, Vec<rerun::components::RotationQuat>),)*
    }
}}

fn setup_meshes(dbg: DebugContext) {
    dbg.log_static(
        "nao",
        &rerun::Transform3D::update_fields().with_axis_length(0.),
    );

    meshes! {($($name:ident, $_:ty)*) => {$(
        let path = concat!("nao/", stringify!($name));

        dbg.log_static(
            path,
            &rerun::Asset3D::from_file_path(concat!("./assets/rerun/nao/", stringify!($name), ".glb"))
                .expect(concat!("Failed to load ", stringify!($), " model"))
                .with_media_type(rerun::MediaType::glb()),
        );
    )*}}
}

fn update_meshes(
    dbg: DebugContext,
    kinematics: Res<Kinematics>,
    pose: Res<RobotPose>,
    orientation: Res<RobotOrientation>,
    cycle: Res<Cycle>,
    mut buffer: Local<TransformBuffer>,
) {
    let pose = pose.to_3d();
    let (robot_to_ground, _) = kinematics.robot_to_ground(orientation.quaternion());

    dbg.log(
        "nao",
        &rerun::Transform3D::from_translation_rotation(
            pose.translation.vector.data.0[0],
            rerun::Quaternion(pose.rotation.coords.data.0[0]),
        ),
    );

    buffer.cycle.push(cycle.0 as i64);

    meshes! {($($name:ident, $space:ty)*) => {$(
        let isometry = kinematics.isometry::<$space, _>().chain(robot_to_ground.as_ref());

        buffer.$name.0.push(isometry.inner.translation.vector.data.0[0].into());
        buffer.$name.1.push(isometry.inner.rotation.coords.data.0[0].into());
    )*}}

    if buffer.cycle.len() >= RERUN_BATCH_SIZE {
        let timeline = rerun::TimeColumn::new_sequence("cycle", std::mem::take(&mut buffer.cycle));

        meshes! {($($name:ident, $_space:ty)*) => {$(

            dbg.send_columns(
                concat!("nao/", stringify!($name)),
                [timeline.clone()],
                Transform3D::update_fields()
                    .with_many_translation(buffer.$name.0.clone())
                    .with_many_quaternion(buffer.$name.1.clone())
                    .columns_of_unit_batches()
                    .expect(concat!("failed to batch up kinematic transforms for", stringify!($name)))
            );

            buffer.$name.0.clear();
            buffer.$name.1.clear();
        )*}}
    }
}
