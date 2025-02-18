use std::f32::consts::FRAC_PI_2;

use nidhogg::types::SingleArmJoints;

/// in rad
const DEFAULT_ROLL: f32 = 0.13;
const ROLL_FACTOR: f32 = 0.4;
const PITCH_FACTOR: f32 = 8.0;

/// Compute joint positions for an arm of the robot based on the
/// translation of the opposite foot.
pub fn swinging_arm(
    hip_roll: f32,
    opposite_foot_x: f32,
    is_left_arm: bool,
) -> SingleArmJoints<f32> {
    let shoulder_roll = DEFAULT_ROLL + ROLL_FACTOR * hip_roll;
    let shoulder_pitch = FRAC_PI_2 - opposite_foot_x * PITCH_FACTOR;

    if is_left_arm {
        SingleArmJoints::builder()
            .shoulder_pitch(shoulder_pitch)
            .shoulder_roll(shoulder_roll)
            .wrist_yaw(-FRAC_PI_2)
            .build()
    } else {
        SingleArmJoints::builder()
            .shoulder_pitch(shoulder_pitch)
            .shoulder_roll(-shoulder_roll)
            .wrist_yaw(FRAC_PI_2)
            .build()
    }
}
