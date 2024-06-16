use nidhogg::types::JointArray;
use std::time::Duration;

/// Performs linear interpolation between two `JointArray<f32>`.
///
/// # Arguments
/// * `current_position` - Starting position.
/// * `target_position` - Final position.
/// * `scalar` - Scalar from 0-1 that indicates what weight to assign to each position.
pub fn lerp(
    current_position: &JointArray<f32>,
    target_position: &JointArray<f32>,
    scalar: f32,
) -> JointArray<f32> {
    current_position
        .clone()
        .zip(target_position.clone())
        .map(|(curr, target)| curr * (1.0 - scalar) + target * scalar)
}

// Checks if the current position has reached the target position with a certain
/// margin of error.
///
/// # Arguments
/// * `current_position` - Position of which you want to check if it has reached a certain
///                        position.
/// * `target_position` - Position of which you want to check if it has been reached.
/// * `error_margin` - Range within which a target position has been reached.
pub fn reached_position(
    current_position: &JointArray<f32>,
    target_position: &JointArray<f32>,
    error_margin: f32,
) -> bool {
    let mut t = current_position
        .clone()
        .zip(target_position.clone())
        .map(|(curr, target)| target - error_margin <= curr && curr <= target + error_margin);

    // Ignore hands.
    t.left_hand = true;
    t.right_hand = true;
    t.all(|elem| elem)
}

/// Calculates the minimum duration that a single movement will have to take
/// based on a given maximum speed.
///
/// # Arguments
/// * `current_position` - Current position of the robot.
/// * `target_position` - Position the robot will move to in the following movement.
/// * `max_speed` - The maximum speed the joints can move at, in joint unit per second.
pub fn get_min_duration(
    current_position: &JointArray<f32>,
    target_position: &JointArray<f32>,
    max_speed: f32,
) -> Duration {
    // calculating the absolute difference between joint values
    let abs_diff = current_position.diff(target_position.clone());

    // getting the joint value which will have to move the farthest
    let max_distance = abs_diff
        .into_iter()
        .fold(f32::MIN, |joint_diff, max_diff| joint_diff.max(*max_diff));

    Duration::from_secs_f32(max_distance / max_speed)
}
