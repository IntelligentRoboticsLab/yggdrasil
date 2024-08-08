#![cfg(feature = "rerun")]
use std::time::Duration;

use miette::{Context, IntoDiagnostic, Result};
use nalgebra::{Isometry3, Point3};
use nidhogg::types::JointArray;
use rerun::{LineStrips3D, Points3D, RecordingStream};
use tracing::info;
use yggdrasil::kinematics::{self, RobotKinematics};
use yggdrasil::motion::walkv3::WalkingEngineV3Module;
use yggdrasil::prelude::*;

fn main() -> Result<()> {
    miette::set_panic_hook();
    tracing_subscriber::fmt().init();

    info!("Setting up rerun...");

    let rec = rerun::RecordingStreamBuilder::new("yggdrasil_walkv3")
        .spawn()
        .into_diagnostic()
        .wrap_err("Failed to initialise rerun")?;

    let app = App::new()
        .add_module(WalkingEngineV3Module)?
        .add_resource(Resource::new(rec))?
        .init_resource::<JointArray<f32>>()?
        .init_resource::<RobotKinematics>()?
        .add_staged_system(SystemStage::Init, update_kinematics)
        .add_system(log_joints)
        .add_staged_system(SystemStage::Write, artificial_delay);

    info!("Starting walking engine");
    app.run()
}

#[system]
pub fn update_kinematics(
    robot_kinematics: &mut RobotKinematics,
    joints: &JointArray<f32>,
) -> Result<()> {
    *robot_kinematics = RobotKinematics::from(joints);
    Ok(())
}

#[system]
fn artificial_delay() -> Result<()> {
    std::thread::sleep(Duration::from_millis(10));
    Ok(())
}

#[system]
fn log_joints(rec: &RecordingStream, kinematics: &RobotKinematics) -> Result<()> {
    let robot = Point3::origin();
    let head = kinematics.head_to_robot.transform_point(&robot);
    let neck = kinematics.neck_to_robot.transform_point(&robot);
    let torso = kinematics.torso_to_robot.transform_point(&robot);

    let left_shoulder = kinematics.left_shoulder_to_robot.transform_point(&robot);
    let right_shoulder = kinematics.right_shoulder_to_robot.transform_point(&robot);

    let left_hip = kinematics.left_hip_to_robot.transform_point(&robot);
    let left_knee = kinematics.left_tibia_to_robot.transform_point(&robot);
    let left_foot = kinematics.left_foot_to_robot.transform_point(&robot);

    let right_hip = kinematics.right_hip_to_robot.transform_point(&robot);
    let right_knee = kinematics.right_tibia_to_robot.transform_point(&robot);
    let right_foot = kinematics.right_foot_to_robot.transform_point(&robot);

    log_joint(rec, "robot", robot)?;
    log_joint(rec, "head", head)?;
    log_joint(rec, "neck", neck)?;
    log_joint(rec, "torso", torso)?;

    log_joint(rec, "left_shoulder", left_shoulder)?;
    log_joint(rec, "right_shoulder", right_shoulder)?;

    log_joint(rec, "left_hip", left_hip)?;
    log_joint(rec, "left_knee", left_knee)?;
    log_joint(rec, "left_foot", left_foot)?;

    log_joint(rec, "right_hip", right_hip)?;
    log_joint(rec, "right_knee", right_knee)?;
    log_joint(rec, "right_foot", right_foot)?;

    rec.log(
        "robot/links",
        &LineStrips3D::new([
            [tuple(head), tuple(neck)],
            [tuple(neck), tuple(torso)],
            [tuple(neck), tuple(left_shoulder)],
            [tuple(neck), tuple(right_shoulder)],
            [tuple(torso), tuple(left_hip)],
            [tuple(left_hip), tuple(left_knee)],
            [tuple(left_knee), tuple(left_foot)],
            [tuple(torso), tuple(right_hip)],
            [tuple(right_hip), tuple(right_knee)],
            [tuple(right_knee), tuple(right_foot)],
        ]),
    )
    .into_diagnostic()?;

    Ok(())
}

fn log_joint(rec: &RecordingStream, name: impl AsRef<str>, joint: Point3<f32>) -> Result<()> {
    rec.log(
        format!("robot/joints/{}", name.as_ref()),
        &Points3D::new([tuple(joint)])
            .with_radii(vec![0.01])
            .with_labels([name.as_ref()]),
    )
    .into_diagnostic()
}

fn tuple(point: Point3<f32>) -> (f32, f32, f32) {
    (point.x, point.y, point.z)
}
