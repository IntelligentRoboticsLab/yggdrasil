use nalgebra::{Isometry3, Rotation3, Translation3, Vector3};
use nidhogg::types::{LeftLegJoints, RightLegJoints};
use std::f32::consts::PI;

use crate::walk::{engine::Side, dnt_walk::FootOffset};

use super::robot_dimensions::{self, ROBOT_TO_LEFT_PELVIS, ROBOT_TO_RIGHT_PELVIS};

#[derive(Clone)]
pub struct FootPosition {
    pub forward: f32,
    pub left: f32,
    pub turn: f32,
    pub hip_height: f32,
    pub foot_lift: f32,
}

pub fn right_leg_angles(foot_position: &FootOffset) -> RightLegJoints<f32> {
    let right_foot_to_torso = foot_to_torso(Isometry3::from(ROBOT_TO_RIGHT_PELVIS), -1f32, &foot_position);
    let torso_to_right_pelvis =
        Isometry3::rotation(Vector3::x() * PI / 4.0) * Translation3::from(-ROBOT_TO_RIGHT_PELVIS);

    let right_foot_to_right_pelvis = torso_to_right_pelvis * right_foot_to_torso;
    let right_hip_yaw_pitch = compute_hip_yaw_pitch(Side::Right, &right_foot_to_right_pelvis);

    let right_pelvis_to_right_hip = Isometry3::rotation(Vector3::z() * -1.0 * right_hip_yaw_pitch);

    let right_foot_to_right_hip = right_pelvis_to_right_hip * right_foot_to_right_pelvis;
    let right_hip_to_right_foot = right_foot_to_right_hip.translation;

    let right_hip_roll_in_hip =
        -1.0 * (-1.0 * right_hip_to_right_foot.y).atan2(-1.0 * right_hip_to_right_foot.z);
    let right_hip_pitch_minus_alpha = (-1.0 * right_hip_to_right_foot.x).atan2(
        (right_hip_to_right_foot.y.powi(2) + right_hip_to_right_foot.z.powi(2)).sqrt()
            * -1.0
            * right_hip_to_right_foot.z.signum(),
    );

    let right_foot_rotation_c2 =
        Isometry3::rotation(Vector3::y() * -1.0 * right_hip_pitch_minus_alpha)
            * Isometry3::rotation(Vector3::x() * -1.0 * right_hip_roll_in_hip)
            * (right_foot_to_right_hip.rotation * Vector3::z());

    let thigh = robot_dimensions::HIP_TO_KNEE.z.abs();
    let tibia = robot_dimensions::KNEE_TO_ANKLE.z.abs();

    let right_foot_height = right_foot_to_right_hip.translation.vector.norm();

    let right_alpha_cos = (thigh.powi(2) + right_foot_height.powi(2) - tibia.powi(2))
        / (2.0 * thigh * right_foot_height);
    let right_beta_cos = (tibia.powi(2) + right_foot_height.powi(2) - thigh.powi(2))
        / (2.0 * tibia * right_foot_height);

    let right_alpha = -1.0 * right_alpha_cos.clamp(-1.0, 1.0).acos();
    let right_beta = -1.0 * right_beta_cos.clamp(-1.0, 1.0).acos();

    RightLegJoints {
        hip_roll: right_hip_roll_in_hip - PI / 4.0,
        hip_pitch: right_hip_pitch_minus_alpha + right_alpha,
        knee_pitch: -right_alpha - right_beta,
        ankle_pitch: right_foot_rotation_c2.x.atan2(right_foot_rotation_c2.z) + right_beta,
        ankle_roll: (-1.0 * right_foot_rotation_c2.y).asin(),
    }
}

pub fn left_leg_angles(foot_position: &FootOffset) -> LeftLegJoints<f32> {
    // hip kinematics
    let left_foot_to_torso = foot_to_torso(Isometry3::from(ROBOT_TO_LEFT_PELVIS), 1f32, &foot_position);
    let torso_to_left_pelvis =
        Isometry3::rotation(Vector3::x() * PI / -4.0) * Translation3::from(-ROBOT_TO_LEFT_PELVIS);

    let left_foot_to_left_pelvis = torso_to_left_pelvis * left_foot_to_torso;
    let left_hip_yaw_pitch = compute_hip_yaw_pitch(Side::Left, &left_foot_to_left_pelvis);

    // leg kinematics:
    // now that we have the actual hip yaw pitch, we can get transformation from pelvis to hip
    let left_pelvis_to_left_hip = Isometry3::rotation(Vector3::z() * left_hip_yaw_pitch);

    // and left foot to left hip
    let left_foot_to_left_hip = left_pelvis_to_left_hip * left_foot_to_left_pelvis;
    let left_hip_to_left_foot = left_foot_to_left_hip.translation;

    let left_hip_roll_in_hip =
        -1.0 * (-1.0 * left_hip_to_left_foot.y).atan2(-1.0 * left_hip_to_left_foot.z);
    let left_hip_pitch_minus_alpha = (-1.0 * left_hip_to_left_foot.x).atan2(
        (left_hip_to_left_foot.y.powi(2) + left_hip_to_left_foot.z.powi(2)).sqrt()
            * -1.0
            * left_hip_to_left_foot.z.signum(),
    );

    let left_foot_rotation_c2 =
        Isometry3::rotation(Vector3::y() * -1.0 * left_hip_pitch_minus_alpha)
            * Isometry3::rotation(Vector3::x() * -1.0 * left_hip_roll_in_hip)
            * (left_foot_to_left_hip.rotation * Vector3::z());

    let thigh = robot_dimensions::HIP_TO_KNEE.z.abs();
    let tibia = robot_dimensions::KNEE_TO_ANKLE.z.abs();

    let left_foot_height = left_foot_to_left_hip.translation.vector.norm();

    let left_alpha_cos = (thigh.powi(2) + left_foot_height.powi(2) - tibia.powi(2))
        / (2.0 * thigh * left_foot_height);
    let left_beta_cos = (tibia.powi(2) + left_foot_height.powi(2) - thigh.powi(2))
        / (2.0 * tibia * left_foot_height);

    let left_alpha = -1.0 * left_alpha_cos.clamp(-1.0, 1.0).acos();
    let left_beta = -1.0 * left_beta_cos.clamp(-1.0, 1.0).acos();

    LeftLegJoints {
        hip_yaw_pitch: left_hip_yaw_pitch,
        hip_roll: left_hip_roll_in_hip + PI / 4.0,
        hip_pitch: left_hip_pitch_minus_alpha + left_alpha,
        knee_pitch: -left_alpha - left_beta,
        ankle_pitch: left_foot_rotation_c2.x.atan2(left_foot_rotation_c2.z) + left_beta,
        ankle_roll: (-1.0 * left_foot_rotation_c2.y).asin(),
    }
}

#[inline]
fn compute_hip_yaw_pitch(side: Side, foot_to_pelvis: &Isometry3<f32>) -> f32 {
    // get vector pointing from pelvis to foot, to compute the angles
    let pelvis_to_foot = foot_to_pelvis.inverse().translation;

    // TODO: diagram
    let foot_roll_in_pelvis = pelvis_to_foot.y.atan2(pelvis_to_foot.z);

    let foot_pitch_in_pelvis = pelvis_to_foot
        .x
        .atan2((pelvis_to_foot.y.powi(2) + pelvis_to_foot.z.powi(2)).sqrt());

    let rotation = Rotation3::new(Vector3::x() * -1.0 * foot_roll_in_pelvis)
        * Rotation3::new(Vector3::y() * foot_pitch_in_pelvis);

    // foot_to_pelvis contains z component, we apply the y component using the rotation computed earlier
    let hip_rotation_c1 = foot_to_pelvis.rotation * (rotation * Vector3::y());

    match side {
        Side::Left => -1.0 * (-1.0 * hip_rotation_c1.x).atan2(hip_rotation_c1.y),
        Side::Right => (-1.0 * hip_rotation_c1.x).atan2(hip_rotation_c1.y),
    }
}

#[inline]
pub fn foot_to_torso(hip_to_robot: Isometry3<f32>, rot_mod: f32, foot_position: &FootOffset) -> Isometry3<f32> {
    let FootOffset {
        forward,
        left,
        turn,
        hip_height,
        foot_lift,
    } = foot_position;
    let rotation = *turn * rot_mod;

    // TODO: use
    let torso_offset = 0.02;

    let foot_translation =
        Isometry3::translation(forward - torso_offset, *left, -hip_height + foot_lift);
    let foot_rotation = Isometry3::rotation(Vector3::z() * rotation);

    hip_to_robot * foot_translation * foot_rotation
}
