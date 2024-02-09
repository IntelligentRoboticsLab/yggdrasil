use nalgebra::{Isometry3, Matrix3x1, Vector3};
use nidhogg::types::{LeftLegJoints, RightLegJoints};
use std::f32::consts::PI;

use super::{robot_dimensions, Left, Right};

/// Compute the leg angles for the given foot positions.
///
/// The foot positions are relative to the robot's torso, and the angles are relative to the robot's
/// pelvis.
pub fn leg_angles(
    left_foot: &super::FootOffset,
    right_foot: &super::FootOffset,
) -> (LeftLegJoints<f32>, RightLegJoints<f32>) {
    let left_foot = left_foot.into_left();
    let right_foot = right_foot.into_right();

    // TODO: Properly use this value.
    // The torso offset is the offset of the torso w.r.t. the pelvis.
    // Currently it's set to a constant 2.5 cm (forward), but it should perhaps be a parameter.
    // Or something that can be set dynamically to balance the robot.
    let torso_offset = 0.025;
    let left_foot_to_left_pelvis = left_foot.to_pelvis(torso_offset);
    let left_hip_yaw_pitch =
        -1.0 * super::SidedFootOffset::<Left>::compute_hip_yaw_pitch(&left_foot_to_left_pelvis);

    let right_foot_to_right_pelvis = right_foot.to_pelvis(torso_offset);
    let right_hip_yaw_pitch =
        super::SidedFootOffset::<Right>::compute_hip_yaw_pitch(&right_foot_to_right_pelvis);

    // the NAO robot has a single hip yaw pitch joint, so we average the two
    let hip_yaw_pitch_combined = (left_hip_yaw_pitch + right_hip_yaw_pitch) / 2.0;

    (
        left_leg_angles(left_foot_to_left_pelvis, hip_yaw_pitch_combined),
        right_leg_angles(right_foot_to_right_pelvis, -hip_yaw_pitch_combined),
    )
}

fn right_leg_angles(
    right_foot_to_right_pelvis: Isometry3<f32>,
    hip_yaw_pitch_combined: f32,
) -> RightLegJoints<f32> {
    let LegJointAngleComponents {
        hip_roll_in_hip,
        hip_pitch_minus_alpha,
        alpha,
        beta,
        foot_rotation_c2,
    } = compute_joint_angles(hip_yaw_pitch_combined, right_foot_to_right_pelvis);

    RightLegJoints {
        hip_roll: hip_roll_in_hip - PI / 4.0,
        hip_pitch: hip_pitch_minus_alpha + alpha,
        knee_pitch: -alpha - beta,
        ankle_pitch: foot_rotation_c2.x.atan2(foot_rotation_c2.z) + beta,
        ankle_roll: (-1.0 * foot_rotation_c2.y).asin(),
    }
}

fn left_leg_angles(
    left_foot_to_left_pelvis: Isometry3<f32>,
    hip_yaw_pitch_combined: f32,
) -> LeftLegJoints<f32> {
    let LegJointAngleComponents {
        hip_roll_in_hip,
        hip_pitch_minus_alpha,
        alpha,
        beta,
        foot_rotation_c2,
    } = compute_joint_angles(hip_yaw_pitch_combined, left_foot_to_left_pelvis);

    LeftLegJoints {
        hip_yaw_pitch: hip_yaw_pitch_combined,
        hip_roll: hip_roll_in_hip + PI / 4.0,
        hip_pitch: hip_pitch_minus_alpha + alpha,
        knee_pitch: -alpha - beta,
        ankle_pitch: foot_rotation_c2.x.atan2(foot_rotation_c2.z) + beta,
        ankle_roll: (-1.0 * foot_rotation_c2.y).asin(),
    }
}

struct LegJointAngleComponents {
    hip_roll_in_hip: f32,
    hip_pitch_minus_alpha: f32,
    alpha: f32,
    beta: f32,
    foot_rotation_c2: Matrix3x1<f32>,
}

#[inline]
fn compute_joint_angles(
    hip_yaw_pitch_combined: f32,
    foot_to_pelvis: Isometry3<f32>,
) -> LegJointAngleComponents {
    let pelvis_to_hip = Isometry3::rotation(Vector3::z() * hip_yaw_pitch_combined);
    let foot_to_hip = pelvis_to_hip * foot_to_pelvis;
    let hip_to_foot = foot_to_hip.translation;

    let hip_roll_in_hip = -1.0 * (-1.0 * hip_to_foot.y).atan2(-1.0 * hip_to_foot.z);
    let hip_pitch_minus_alpha = (-1.0 * hip_to_foot.x).atan2(
        (hip_to_foot.y.powi(2) + hip_to_foot.z.powi(2)).sqrt() * -1.0 * hip_to_foot.z.signum(),
    );

    let foot_rotation_c2 = Isometry3::rotation(Vector3::y() * -1.0 * hip_pitch_minus_alpha)
        * Isometry3::rotation(Vector3::x() * -1.0 * hip_roll_in_hip)
        * (foot_to_hip.rotation * Vector3::z());

    let thigh = robot_dimensions::HIP_TO_KNEE.z.abs();
    let tibia = robot_dimensions::KNEE_TO_ANKLE.z.abs();

    let foot_height = foot_to_hip.translation.vector.norm();

    let alpha_cos =
        (thigh.powi(2) + foot_height.powi(2) - tibia.powi(2)) / (2.0 * thigh * foot_height);
    let beta_cos =
        (tibia.powi(2) + foot_height.powi(2) - thigh.powi(2)) / (2.0 * tibia * foot_height);

    let alpha = -1.0 * alpha_cos.clamp(-1.0, 1.0).acos();
    let beta = -1.0 * beta_cos.clamp(-1.0, 1.0).acos();

    LegJointAngleComponents {
        hip_roll_in_hip,
        hip_pitch_minus_alpha,
        alpha,
        beta,
        foot_rotation_c2,
    }
}
