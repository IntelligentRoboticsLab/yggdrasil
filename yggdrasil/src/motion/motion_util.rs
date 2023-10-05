use nidhogg::types::JointArray;

pub trait MotionUtilExt<T: Clone> {
    fn all<Predicate>(&self, predicate: Predicate) -> bool
    where
        Predicate: FnMut(T) -> bool;

    fn any<Predicate>(&self, predicate: Predicate) -> bool
    where
        Predicate: FnMut(T) -> bool;
}

impl<T: Clone> MotionUtilExt<T> for JointArray<T> {
    /// Checks if all elements of a joint array satisfy a certain condition.
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::JointArray;
    ///
    /// let mut t1: JointArray<i32> = JointArray::default();
    /// assert_eq!(t1.clone().all(|elem| elem > -1), true);
    ///
    /// t1.right_hand = -2;
    /// assert_eq!(t1.all(|elem| elem > -1), false);
    /// ```
    fn all<Predicate>(&self, mut f: Predicate) -> bool
    where
        Predicate: FnMut(T) -> bool,
    {
        !self.any(|elem| !f(elem))
    }

    /// Checks if any elements of a joint array satisfy a certain condition.
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::JointArray;
    ///
    /// let mut t1: JointArray<i32> = JointArray::default();
    /// assert_eq!(t1.clone().any(|elem| elem > 2), false);
    ///
    /// t1.head_pitch = 3;
    /// assert_eq!(t1.any(|elem| elem > 2), true);
    /// ```
    fn any<Predicate>(&self, predicate: Predicate) -> bool
    where
        Predicate: FnMut(T) -> bool,
    {
        let t = self.clone().map(predicate);

        t.head_yaw
            || t.head_pitch
            || t.left_shoulder_pitch
            || t.left_shoulder_roll
            || t.left_elbow_yaw
            || t.left_elbow_roll
            || t.left_wrist_yaw
            || t.left_hip_yaw_pitch
            || t.left_hip_roll
            || t.left_hip_pitch
            || t.left_knee_pitch
            || t.left_ankle_pitch
            || t.left_ankle_roll
            || t.right_shoulder_pitch
            || t.right_shoulder_roll
            || t.right_elbow_yaw
            || t.right_elbow_roll
            || t.right_wrist_yaw
            || t.right_hip_roll
            || t.right_hip_pitch
            || t.right_knee_pitch
            || t.right_ankle_pitch
            || t.right_ankle_roll
            || t.left_hand
            || t.right_hand
    }
}

/// Performs linear interpolation between two `JointArray<f32>`.
///
/// # Arguments
///
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
