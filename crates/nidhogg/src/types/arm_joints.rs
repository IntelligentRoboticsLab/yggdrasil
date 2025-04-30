use nidhogg_derive::{Builder, Filler};

use std::ops::{Add, Div, Mul, Sub};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::FillExt;

/// Wrapper struct containing the joints for a single arm of the robot.
#[derive(Builder, Clone, Debug, Default, Filler, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SingleArmJoints<T> {
    pub shoulder_pitch: T,
    pub shoulder_roll: T,
    pub elbow_yaw: T,
    pub elbow_roll: T,
    pub wrist_yaw: T,
    pub hand: T,
}

impl<T> SingleArmJoints<T> {
    /// Transforms each element in the [`SingleArmJoints`] using the provided closure `f`,
    /// producing a new [`SingleArmJoints`] with the transformed values.
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::SingleArmJoints;
    ///
    /// let joints = SingleArmJoints::<u32>::default();
    ///
    /// let transformed = joints.map(|x| x + 1);
    /// ```
    pub fn map<F, U>(self, mut f: F) -> SingleArmJoints<U>
    where
        F: FnMut(T) -> U,
    {
        SingleArmJoints {
            shoulder_pitch: f(self.shoulder_pitch),
            shoulder_roll: f(self.shoulder_roll),
            elbow_yaw: f(self.elbow_yaw),
            elbow_roll: f(self.elbow_roll),
            wrist_yaw: f(self.wrist_yaw),
            hand: f(self.hand),
        }
    }

    /// Zips two [`SingleArmJoints`] instances element-wise, creating a new [`SingleArmJoints`]
    /// containing tuples of corresponding elements from the two arrays.
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::SingleArmJoints;
    /// use nidhogg::types::FillExt;
    ///
    /// let zipped = SingleArmJoints::<u32>::default().zip(SingleArmJoints::<u32>::default());
    ///
    /// assert_eq!(zipped, SingleArmJoints::<(u32, u32)>::fill((0_u32, 0_u32)));
    /// ```
    pub fn zip<U>(self, other: SingleArmJoints<U>) -> SingleArmJoints<(T, U)> {
        SingleArmJoints {
            shoulder_pitch: (self.shoulder_pitch, other.shoulder_pitch),
            shoulder_roll: (self.shoulder_roll, other.shoulder_roll),
            elbow_yaw: (self.elbow_yaw, other.elbow_yaw),
            elbow_roll: (self.elbow_roll, other.elbow_roll),
            wrist_yaw: (self.wrist_yaw, other.wrist_yaw),
            hand: (self.hand, other.hand),
        }
    }

    /// Return an iterator over references to the elements of the [`SingleArmJoints`].
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::SingleArmJoints;
    ///
    /// let joints = SingleArmJoints::<f32>::default();
    /// for joint in joints.iter() {
    ///     assert_eq!(joint, 0.0);
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        [
            &self.shoulder_pitch,
            &self.shoulder_roll,
            &self.elbow_yaw,
            &self.elbow_roll,
            &self.wrist_yaw,
            &self.hand,
        ]
        .into_iter()
    }
}

impl<T> Add<T> for SingleArmJoints<T>
where
    T: Add<Output = T> + Clone,
{
    type Output = Self;

    fn add(self, rhs: T) -> Self {
        Self {
            shoulder_pitch: self.shoulder_pitch + rhs.clone(),
            shoulder_roll: self.shoulder_roll + rhs.clone(),
            elbow_yaw: self.elbow_yaw + rhs.clone(),
            elbow_roll: self.elbow_roll + rhs.clone(),
            wrist_yaw: self.wrist_yaw + rhs.clone(),
            hand: self.hand + rhs,
        }
    }
}

impl<T> Add for SingleArmJoints<T>
where
    T: Add<Output = T>,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            shoulder_pitch: self.shoulder_pitch + rhs.shoulder_pitch,
            shoulder_roll: self.shoulder_roll + rhs.shoulder_roll,
            elbow_yaw: self.elbow_yaw + rhs.elbow_yaw,
            elbow_roll: self.elbow_roll + rhs.elbow_roll,
            wrist_yaw: self.wrist_yaw + rhs.wrist_yaw,
            hand: self.hand + rhs.hand,
        }
    }
}

impl<T> Sub<T> for SingleArmJoints<T>
where
    T: Sub<Output = T> + Clone,
{
    type Output = Self;

    fn sub(self, rhs: T) -> Self {
        Self {
            shoulder_pitch: self.shoulder_pitch - rhs.clone(),
            shoulder_roll: self.shoulder_roll - rhs.clone(),
            elbow_yaw: self.elbow_yaw - rhs.clone(),
            elbow_roll: self.elbow_roll - rhs.clone(),
            wrist_yaw: self.wrist_yaw - rhs.clone(),
            hand: self.hand - rhs,
        }
    }
}

impl<T> Sub for SingleArmJoints<T>
where
    T: Sub<Output = T>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            shoulder_pitch: self.shoulder_pitch - rhs.shoulder_pitch,
            shoulder_roll: self.shoulder_roll - rhs.shoulder_roll,
            elbow_yaw: self.elbow_yaw - rhs.elbow_yaw,
            elbow_roll: self.elbow_roll - rhs.elbow_roll,
            wrist_yaw: self.wrist_yaw - rhs.wrist_yaw,
            hand: self.hand - rhs.hand,
        }
    }
}

impl<T> Mul<T> for SingleArmJoints<T>
where
    T: Mul<Output = T> + Clone,
{
    type Output = Self;

    fn mul(self, rhs: T) -> Self {
        Self {
            shoulder_pitch: self.shoulder_pitch * rhs.clone(),
            shoulder_roll: self.shoulder_roll * rhs.clone(),
            elbow_yaw: self.elbow_yaw * rhs.clone(),
            elbow_roll: self.elbow_roll * rhs.clone(),
            wrist_yaw: self.wrist_yaw * rhs.clone(),
            hand: self.hand * rhs,
        }
    }
}

impl<T> Mul for SingleArmJoints<T>
where
    T: Mul<Output = T>,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self {
            shoulder_pitch: self.shoulder_pitch * rhs.shoulder_pitch,
            shoulder_roll: self.shoulder_roll * rhs.shoulder_roll,
            elbow_yaw: self.elbow_yaw * rhs.elbow_yaw,
            elbow_roll: self.elbow_roll * rhs.elbow_roll,
            wrist_yaw: self.wrist_yaw * rhs.wrist_yaw,
            hand: self.hand * rhs.hand,
        }
    }
}

impl<T> Div<T> for SingleArmJoints<T>
where
    T: Div<Output = T> + Clone,
{
    type Output = Self;

    fn div(self, rhs: T) -> Self {
        Self {
            shoulder_pitch: self.shoulder_pitch / rhs.clone(),
            shoulder_roll: self.shoulder_roll / rhs.clone(),
            elbow_yaw: self.elbow_yaw / rhs.clone(),
            elbow_roll: self.elbow_roll / rhs.clone(),
            wrist_yaw: self.wrist_yaw / rhs.clone(),
            hand: self.hand / rhs,
        }
    }
}

impl<T> Div for SingleArmJoints<T>
where
    T: Div<Output = T>,
{
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        Self {
            shoulder_pitch: self.shoulder_pitch / rhs.shoulder_pitch,
            shoulder_roll: self.shoulder_roll / rhs.shoulder_roll,
            elbow_yaw: self.elbow_yaw / rhs.elbow_yaw,
            elbow_roll: self.elbow_roll / rhs.elbow_roll,
            wrist_yaw: self.wrist_yaw / rhs.wrist_yaw,
            hand: self.hand / rhs.hand,
        }
    }
}

/// Type definition for the left arm joints of the robot.
/// Introduced for api consistency with [`LeftLegJoints`].
pub type LeftArmJoints<T> = SingleArmJoints<T>;

/// Type definition for the right arm joints of the robot.
/// Introduced for api consistency with [`RightLegJoints`].
pub type RightArmJoints<T> = SingleArmJoints<T>;

/// Wrapper struct containing the arm joints of the robot.
#[derive(Builder, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ArmJoints<T> {
    pub left_arm: SingleArmJoints<T>,
    pub right_arm: SingleArmJoints<T>,
}

impl<T> ArmJoints<T> {
    /// Transforms each element in the [`ArmJoints`] using the provided closure `f`,
    /// producing a new [`ArmJoints`] with the transformed values.
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::ArmJoints;
    ///
    /// let joints = ArmJoints::<u32>::default();
    ///
    /// let transformed = joints.map(|x| x + 1);
    /// ```
    pub fn map<F, U>(self, mut f: F) -> ArmJoints<U>
    where
        F: FnMut(T) -> U,
    {
        ArmJoints {
            left_arm: self.left_arm.map(&mut f),
            right_arm: self.right_arm.map(&mut f),
        }
    }

    /// Zips two [`ArmJoints`] instances element-wise, creating a new [`ArmJoints`]
    /// containing tuples of corresponding elements from the two arrays.
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::ArmJoints;
    /// use nidhogg::types::FillExt;
    ///
    /// let zipped = ArmJoints::<u32>::default().zip(ArmJoints::<u32>::default());
    ///
    /// assert_eq!(zipped, ArmJoints::<(u32, u32)>::fill((0_u32, 0_u32)));
    /// ```
    pub fn zip<U>(self, other: ArmJoints<U>) -> ArmJoints<(T, U)> {
        ArmJoints {
            left_arm: self.left_arm.zip(other.left_arm),
            right_arm: self.right_arm.zip(other.right_arm),
        }
    }
}

impl<T: Clone> FillExt<T> for ArmJoints<T> {
    fn fill(value: T) -> ArmJoints<T> {
        ArmJoints {
            left_arm: LeftArmJoints::fill(value.clone()),
            right_arm: RightArmJoints::fill(value.clone()),
        }
    }
}

impl<T> Add<T> for ArmJoints<T>
where
    T: Add<Output = T> + Clone,
{
    type Output = Self;

    fn add(self, rhs: T) -> Self {
        Self {
            left_arm: self.left_arm + rhs.clone(),
            right_arm: self.right_arm + rhs,
        }
    }
}

impl<T> Add for ArmJoints<T>
where
    T: Add<Output = T>,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            left_arm: self.left_arm + rhs.left_arm,
            right_arm: self.right_arm + rhs.right_arm,
        }
    }
}

impl<T> Sub<T> for ArmJoints<T>
where
    T: Sub<Output = T> + Clone,
{
    type Output = Self;

    fn sub(self, rhs: T) -> Self {
        Self {
            left_arm: self.left_arm - rhs.clone(),
            right_arm: self.right_arm - rhs,
        }
    }
}

impl<T> Sub for ArmJoints<T>
where
    T: Sub<Output = T>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            left_arm: self.left_arm - rhs.left_arm,
            right_arm: self.right_arm - rhs.right_arm,
        }
    }
}

impl<T> Mul<T> for ArmJoints<T>
where
    T: Mul<Output = T> + Clone,
{
    type Output = Self;

    fn mul(self, rhs: T) -> Self {
        Self {
            left_arm: self.left_arm * rhs.clone(),
            right_arm: self.right_arm * rhs,
        }
    }
}

impl<T> Mul for ArmJoints<T>
where
    T: Mul<Output = T>,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self {
            left_arm: self.left_arm * rhs.left_arm,
            right_arm: self.right_arm * rhs.right_arm,
        }
    }
}

impl<T> Div<T> for ArmJoints<T>
where
    T: Div<Output = T> + Clone,
{
    type Output = Self;

    fn div(self, rhs: T) -> Self {
        Self {
            left_arm: self.left_arm / rhs.clone(),
            right_arm: self.right_arm / rhs,
        }
    }
}

impl<T> Div for ArmJoints<T>
where
    T: Div<Output = T>,
{
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        Self {
            left_arm: self.left_arm / rhs.left_arm,
            right_arm: self.right_arm / rhs.right_arm,
        }
    }
}
