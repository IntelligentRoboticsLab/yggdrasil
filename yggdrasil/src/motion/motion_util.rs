use crate::motion::motion_types::InterpolationType;
use nidhogg::types::JointArray;
use num::pow;
use std::time::Duration;

/// Selects and executes the chosen interpolation method between two `JointArray<f32>`.
/// Returns a jointarray containing the appropriate joint values at the given
/// time.
///
/// # Notes
/// - The value `t` is contained in the set (0, 1). This represents the ratio
/// of the movement duration that has elapsed. For example: 0 is equivalent to the start of the movement,
/// 1 is equivalent to the end of the movement and 0.5 is equivalent to the halfway point of the movement.
/// - Currently the function only applies a standard bezier interpolation within the "SmoothInOut"
///   interpolation type. However, there will be an option to request a custom bezier interpolation method.
///
/// # Arguments
/// * `previous_position` - Previous keyframe position of the robot.
/// * `target_position` - Target keyframe position the robot wants to move to.
/// * `t` - The ratio of the movement duration that has elapsed. Within (0, 1).
/// * `interpolation_type` - Interpolation type to be applied.
pub fn interpolate_jointarrays(
    previous_position: &JointArray<f32>,
    target_position: &JointArray<f32>,
    t: f32,
    interpolation_type: &InterpolationType,
) -> JointArray<f32> {
    let ratio = match interpolation_type {
        InterpolationType::Linear => t,
        InterpolationType::SmoothInOut => jointarray_cubic_bezier(0.0, 1.0, t),
    };

    previous_position
        .clone()
        .zip(target_position.clone())
        .map(|(curr, target)| curr * (1.0 - ratio) + target * ratio)
}

/// Function for applying cubic bezier interpolation on a scalar.
/// This means that this bezier curve is considered to be 1-dimensional.
///
/// # Arguments
/// * `p1` - First control point between the values of 0 and 1.
/// * `p2` - Second control point between the values of 0 and 1.
/// * `t` - Variable value t.
pub fn jointarray_cubic_bezier(p1: f32, p2: f32, t: f32) -> f32 {
    pow(3.0 * (1.0 - t), 2) * t * p1 + (3.0 * (1.0 - t) * pow(t, 2)) * p2 + pow(t, 3)
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
        .fold(std::f32::MIN, |joint_diff, max_diff| {
            joint_diff.max(*max_diff)
        });

    Duration::from_secs_f32(max_distance / max_speed)
}
