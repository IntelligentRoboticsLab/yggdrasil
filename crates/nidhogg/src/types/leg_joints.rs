use nidhogg_derive::{Builder, Filler};

use std::ops::{Add, Div, Mul, Sub};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use super::FillExt;

/// Wrapper struct containing the left leg joints of the robot.
#[derive(Builder, Clone, Debug, Default, Filler, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LeftLegJoints<T> {
    pub hip_yaw_pitch: T,
    pub hip_roll: T,
    pub hip_pitch: T,
    pub knee_pitch: T,
    pub ankle_pitch: T,
    pub ankle_roll: T,
}

impl<T> LeftLegJoints<T> {
    /// Transforms each element in the [`LeftLegJoints`] using the provided closure `f`,
    /// producing a new [`LeftLegJoints`] with the transformed values.
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::LeftLegJoints;
    /// use nidhogg::types::FillExt;
    ///
    /// let joints = LeftLegJoints::<u32>::default();
    ///
    /// let transformed = joints.map(|x| x + 1);
    ///
    /// assert_eq!(transformed, LeftLegJoints::fill(1));
    /// ```
    pub fn map<F, U>(self, mut f: F) -> LeftLegJoints<U>
    where
        F: FnMut(T) -> U,
    {
        LeftLegJoints {
            hip_yaw_pitch: f(self.hip_yaw_pitch),
            hip_roll: f(self.hip_roll),
            hip_pitch: f(self.hip_pitch),
            knee_pitch: f(self.knee_pitch),
            ankle_pitch: f(self.ankle_pitch),
            ankle_roll: f(self.ankle_roll),
        }
    }

    /// Zips two [`LeftLegJoints`] instances element-wise, creating a new [`LeftLegJoints`]
    /// containing tuples of corresponding elements from the two arrays.
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::LeftLegJoints;
    /// use nidhogg::types::FillExt;
    ///
    /// let zipped = LeftLegJoints::<u32>::default().zip(LeftLegJoints::<u32>::default());
    ///
    /// assert_eq!(zipped, LeftLegJoints::<(u32, u32)>::fill((0_u32, 0_u32)));
    /// ```
    pub fn zip<U>(self, other: LeftLegJoints<U>) -> LeftLegJoints<(T, U)> {
        LeftLegJoints {
            hip_yaw_pitch: (self.hip_yaw_pitch, other.hip_yaw_pitch),
            hip_roll: (self.hip_roll, other.hip_roll),
            hip_pitch: (self.hip_pitch, other.hip_pitch),
            knee_pitch: (self.knee_pitch, other.knee_pitch),
            ankle_pitch: (self.ankle_pitch, other.ankle_pitch),
            ankle_roll: (self.ankle_roll, other.ankle_roll),
        }
    }

    /// Return an iterator over references to the elements of the [`LeftLegJoints`].
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::LeftLegJoints;
    ///
    /// let joints = LeftLegJoints::<f32>::default();
    /// for joint in joints.iter() {
    ///     assert_eq!(joint, 0.0);
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        [
            &self.hip_yaw_pitch,
            &self.hip_roll,
            &self.hip_pitch,
            &self.knee_pitch,
            &self.ankle_pitch,
            &self.ankle_roll,
        ]
        .into_iter()
    }
}

impl<T> Add<T> for LeftLegJoints<T>
where
    T: Add<Output = T> + Clone,
{
    type Output = Self;

    fn add(self, rhs: T) -> Self {
        Self {
            hip_yaw_pitch: self.hip_yaw_pitch + rhs.clone(),
            hip_roll: self.hip_roll + rhs.clone(),
            hip_pitch: self.hip_pitch + rhs.clone(),
            knee_pitch: self.knee_pitch + rhs.clone(),
            ankle_pitch: self.ankle_pitch + rhs.clone(),
            ankle_roll: self.ankle_roll + rhs,
        }
    }
}

impl<T> Add for LeftLegJoints<T>
where
    T: Add<Output = T>,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            hip_yaw_pitch: self.hip_yaw_pitch + rhs.hip_yaw_pitch,
            hip_roll: self.hip_roll + rhs.hip_roll,
            hip_pitch: self.hip_pitch + rhs.hip_pitch,
            knee_pitch: self.knee_pitch + rhs.knee_pitch,
            ankle_pitch: self.ankle_pitch + rhs.ankle_pitch,
            ankle_roll: self.ankle_roll + rhs.ankle_roll,
        }
    }
}

impl<T> Sub<T> for LeftLegJoints<T>
where
    T: Sub<Output = T> + Clone,
{
    type Output = Self;

    fn sub(self, rhs: T) -> Self {
        Self {
            hip_yaw_pitch: self.hip_yaw_pitch - rhs.clone(),
            hip_roll: self.hip_roll - rhs.clone(),
            hip_pitch: self.hip_pitch - rhs.clone(),
            knee_pitch: self.knee_pitch - rhs.clone(),
            ankle_pitch: self.ankle_pitch - rhs.clone(),
            ankle_roll: self.ankle_roll - rhs,
        }
    }
}

impl<T> Sub for LeftLegJoints<T>
where
    T: Sub<Output = T>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            hip_yaw_pitch: self.hip_yaw_pitch - rhs.hip_yaw_pitch,
            hip_roll: self.hip_roll - rhs.hip_roll,
            hip_pitch: self.hip_pitch - rhs.hip_pitch,
            knee_pitch: self.knee_pitch - rhs.knee_pitch,
            ankle_pitch: self.ankle_pitch - rhs.ankle_pitch,
            ankle_roll: self.ankle_roll - rhs.ankle_roll,
        }
    }
}

impl<T> Mul<T> for LeftLegJoints<T>
where
    T: Mul<Output = T> + Clone,
{
    type Output = Self;

    fn mul(self, rhs: T) -> Self {
        Self {
            hip_yaw_pitch: self.hip_yaw_pitch * rhs.clone(),
            hip_roll: self.hip_roll * rhs.clone(),
            hip_pitch: self.hip_pitch * rhs.clone(),
            knee_pitch: self.knee_pitch * rhs.clone(),
            ankle_pitch: self.ankle_pitch * rhs.clone(),
            ankle_roll: self.ankle_roll * rhs,
        }
    }
}

impl<T> Mul for LeftLegJoints<T>
where
    T: Mul<Output = T>,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self {
            hip_yaw_pitch: self.hip_yaw_pitch * rhs.hip_yaw_pitch,
            hip_roll: self.hip_roll * rhs.hip_roll,
            hip_pitch: self.hip_pitch * rhs.hip_pitch,
            knee_pitch: self.knee_pitch * rhs.knee_pitch,
            ankle_pitch: self.ankle_pitch * rhs.ankle_pitch,
            ankle_roll: self.ankle_roll * rhs.ankle_roll,
        }
    }
}

impl<T> Div<T> for LeftLegJoints<T>
where
    T: Div<Output = T> + Clone,
{
    type Output = Self;

    fn div(self, rhs: T) -> Self {
        Self {
            hip_yaw_pitch: self.hip_yaw_pitch / rhs.clone(),
            hip_roll: self.hip_roll / rhs.clone(),
            hip_pitch: self.hip_pitch / rhs.clone(),
            knee_pitch: self.knee_pitch / rhs.clone(),
            ankle_pitch: self.ankle_pitch / rhs.clone(),
            ankle_roll: self.ankle_roll / rhs,
        }
    }
}

impl<T> Div for LeftLegJoints<T>
where
    T: Div<Output = T>,
{
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        Self {
            hip_yaw_pitch: self.hip_yaw_pitch / rhs.hip_yaw_pitch,
            hip_roll: self.hip_roll / rhs.hip_roll,
            hip_pitch: self.hip_pitch / rhs.hip_pitch,
            knee_pitch: self.knee_pitch / rhs.knee_pitch,
            ankle_pitch: self.ankle_pitch / rhs.ankle_pitch,
            ankle_roll: self.ankle_roll / rhs.ankle_roll,
        }
    }
}

/// Wrapper struct containing right left leg joints of the robot.
#[derive(Builder, Clone, Debug, Default, Filler, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RightLegJoints<T> {
    // This value does not exist
    // pub hip_yaw_pitch: T,
    pub hip_roll: T,
    pub hip_pitch: T,
    pub knee_pitch: T,
    pub ankle_pitch: T,
    pub ankle_roll: T,
}

impl<T> RightLegJoints<T> {
    /// Transforms each element in the [`RightLegJoints`] using the provided closure `f`,
    /// producing a new [`RightLegJoints`] with the transformed values.
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::RightLegJoints;
    /// use nidhogg::types::FillExt;
    ///
    /// let joints = RightLegJoints::<u32>::default();
    ///
    /// let transformed = joints.map(|x| x + 1);
    ///
    /// assert_eq!(transformed, RightLegJoints::<u32>::fill(1));
    /// ```
    pub fn map<F, U>(self, mut f: F) -> RightLegJoints<U>
    where
        F: FnMut(T) -> U,
    {
        RightLegJoints {
            hip_roll: f(self.hip_roll),
            hip_pitch: f(self.hip_pitch),
            knee_pitch: f(self.knee_pitch),
            ankle_pitch: f(self.ankle_pitch),
            ankle_roll: f(self.ankle_roll),
        }
    }

    /// Zips two [`RightLegJoints`] instances element-wise, creating a new [`RightLegJoints`]
    /// containing tuples of corresponding elements from the two arrays.
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::RightLegJoints;
    /// use nidhogg::types::FillExt;
    ///
    /// let zipped = RightLegJoints::<u32>::default().zip(RightLegJoints::<u32>::default());
    ///
    /// assert_eq!(zipped, RightLegJoints::<(u32, u32)>::fill((0_u32, 0_u32)));
    /// ```
    pub fn zip<U>(self, other: RightLegJoints<U>) -> RightLegJoints<(T, U)> {
        RightLegJoints {
            hip_roll: (self.hip_roll, other.hip_roll),
            hip_pitch: (self.hip_pitch, other.hip_pitch),
            knee_pitch: (self.knee_pitch, other.knee_pitch),
            ankle_pitch: (self.ankle_pitch, other.ankle_pitch),
            ankle_roll: (self.ankle_roll, other.ankle_roll),
        }
    }

    /// Return an iterator over references to the elements of the [`RightLegJoints`].
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::RightLegJoints;
    ///
    /// let joints = RightLegJoints::<f32>::default();
    /// for joint in joints.iter() {
    ///     assert_eq!(joint, 0.0);
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        [
            &self.hip_roll,
            &self.hip_pitch,
            &self.knee_pitch,
            &self.ankle_pitch,
            &self.ankle_roll,
        ]
        .into_iter()
    }
}

impl<T> Add<T> for RightLegJoints<T>
where
    T: Add<Output = T> + Clone,
{
    type Output = Self;

    fn add(self, rhs: T) -> Self {
        Self {
            hip_roll: self.hip_roll + rhs.clone(),
            hip_pitch: self.hip_pitch + rhs.clone(),
            knee_pitch: self.knee_pitch + rhs.clone(),
            ankle_pitch: self.ankle_pitch + rhs.clone(),
            ankle_roll: self.ankle_roll + rhs,
        }
    }
}

impl<T> Add for RightLegJoints<T>
where
    T: Add<Output = T>,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            hip_roll: self.hip_roll + rhs.hip_roll,
            hip_pitch: self.hip_pitch + rhs.hip_pitch,
            knee_pitch: self.knee_pitch + rhs.knee_pitch,
            ankle_pitch: self.ankle_pitch + rhs.ankle_pitch,
            ankle_roll: self.ankle_roll + rhs.ankle_roll,
        }
    }
}

impl<T> Sub<T> for RightLegJoints<T>
where
    T: Sub<Output = T> + Clone,
{
    type Output = Self;

    fn sub(self, rhs: T) -> Self {
        Self {
            hip_roll: self.hip_roll - rhs.clone(),
            hip_pitch: self.hip_pitch - rhs.clone(),
            knee_pitch: self.knee_pitch - rhs.clone(),
            ankle_pitch: self.ankle_pitch - rhs.clone(),
            ankle_roll: self.ankle_roll - rhs,
        }
    }
}

impl<T> Sub for RightLegJoints<T>
where
    T: Sub<Output = T>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            hip_roll: self.hip_roll - rhs.hip_roll,
            hip_pitch: self.hip_pitch - rhs.hip_pitch,
            knee_pitch: self.knee_pitch - rhs.knee_pitch,
            ankle_pitch: self.ankle_pitch - rhs.ankle_pitch,
            ankle_roll: self.ankle_roll - rhs.ankle_roll,
        }
    }
}

impl<T> Mul<T> for RightLegJoints<T>
where
    T: Mul<Output = T> + Clone,
{
    type Output = Self;

    fn mul(self, rhs: T) -> Self {
        Self {
            hip_roll: self.hip_roll * rhs.clone(),
            hip_pitch: self.hip_pitch * rhs.clone(),
            knee_pitch: self.knee_pitch * rhs.clone(),
            ankle_pitch: self.ankle_pitch * rhs.clone(),
            ankle_roll: self.ankle_roll * rhs,
        }
    }
}

impl<T> Mul for RightLegJoints<T>
where
    T: Mul<Output = T>,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self {
            hip_roll: self.hip_roll * rhs.hip_roll,
            hip_pitch: self.hip_pitch * rhs.hip_pitch,
            knee_pitch: self.knee_pitch * rhs.knee_pitch,
            ankle_pitch: self.ankle_pitch * rhs.ankle_pitch,
            ankle_roll: self.ankle_roll * rhs.ankle_roll,
        }
    }
}

impl<T> Div<T> for RightLegJoints<T>
where
    T: Div<Output = T> + Clone,
{
    type Output = Self;

    fn div(self, rhs: T) -> Self {
        Self {
            hip_roll: self.hip_roll / rhs.clone(),
            hip_pitch: self.hip_pitch / rhs.clone(),
            knee_pitch: self.knee_pitch / rhs.clone(),
            ankle_pitch: self.ankle_pitch / rhs.clone(),
            ankle_roll: self.ankle_roll / rhs,
        }
    }
}

impl<T> Div for RightLegJoints<T>
where
    T: Div<Output = T>,
{
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        Self {
            hip_roll: self.hip_roll / rhs.hip_roll,
            hip_pitch: self.hip_pitch / rhs.hip_pitch,
            knee_pitch: self.knee_pitch / rhs.knee_pitch,
            ankle_pitch: self.ankle_pitch / rhs.ankle_pitch,
            ankle_roll: self.ankle_roll / rhs.ankle_roll,
        }
    }
}

/// Wrapper struct containing joint values for both legs of the robot.
#[derive(Builder, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct LegJoints<T> {
    pub left_leg: LeftLegJoints<T>,
    pub right_leg: RightLegJoints<T>,
}

impl<T> LegJoints<T> {
    /// Transforms each element in the [`LegJoints`] using the provided closure `f`,
    /// producing a new [`LegJoints`] with the transformed values.
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::LegJoints;
    /// use nidhogg::types::FillExt;
    ///
    /// let joints = LegJoints::<u32>::default();
    ///
    /// let transformed = joints.map(|x| x + 1);
    ///
    /// assert_eq!(transformed, LegJoints::fill(1));
    /// ```
    pub fn map<F, U>(self, mut f: F) -> LegJoints<U>
    where
        F: FnMut(T) -> U,
    {
        LegJoints {
            left_leg: self.left_leg.map(&mut f),
            right_leg: self.right_leg.map(&mut f),
        }
    }

    /// Zips two [`LegJoints`] instances element-wise, creating a new [`LegJoints`]
    /// containing tuples of corresponding elements from the two arrays.
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::LegJoints;
    /// use nidhogg::types::FillExt;
    ///
    /// let zipped = LegJoints::<u32>::default().zip(LegJoints::<u32>::default());
    ///
    /// assert_eq!(zipped, LegJoints::<(u32, u32)>::fill((0_u32, 0_u32)));
    /// ```
    pub fn zip<U>(self, other: LegJoints<U>) -> LegJoints<(T, U)> {
        LegJoints {
            left_leg: self.left_leg.zip(other.left_leg),
            right_leg: self.right_leg.zip(other.right_leg),
        }
    }
}

impl<T: Clone> FillExt<T> for LegJoints<T> {
    fn fill(value: T) -> LegJoints<T> {
        LegJoints {
            left_leg: LeftLegJoints::fill(value.clone()),
            right_leg: RightLegJoints::fill(value.clone()),
        }
    }
}

impl<T> Add<T> for LegJoints<T>
where
    T: Add<Output = T> + Clone,
{
    type Output = Self;

    fn add(self, rhs: T) -> Self {
        Self {
            left_leg: self.left_leg + rhs.clone(),
            right_leg: self.right_leg + rhs,
        }
    }
}

impl<T> Add for LegJoints<T>
where
    T: Add<Output = T>,
{
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        Self {
            left_leg: self.left_leg + rhs.left_leg,
            right_leg: self.right_leg + rhs.right_leg,
        }
    }
}

impl<T> Sub<T> for LegJoints<T>
where
    T: Sub<Output = T> + Clone,
{
    type Output = Self;

    fn sub(self, rhs: T) -> Self {
        Self {
            left_leg: self.left_leg - rhs.clone(),
            right_leg: self.right_leg - rhs,
        }
    }
}

impl<T> Sub for LegJoints<T>
where
    T: Sub<Output = T>,
{
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        Self {
            left_leg: self.left_leg - rhs.left_leg,
            right_leg: self.right_leg - rhs.right_leg,
        }
    }
}

impl<T> Mul<T> for LegJoints<T>
where
    T: Mul<Output = T> + Clone,
{
    type Output = Self;

    fn mul(self, rhs: T) -> Self {
        Self {
            left_leg: self.left_leg * rhs.clone(),
            right_leg: self.right_leg * rhs,
        }
    }
}

impl<T> Mul for LegJoints<T>
where
    T: Mul<Output = T>,
{
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        Self {
            left_leg: self.left_leg * rhs.left_leg,
            right_leg: self.right_leg * rhs.right_leg,
        }
    }
}

impl<T> Div<T> for LegJoints<T>
where
    T: Div<Output = T> + Clone,
{
    type Output = Self;

    fn div(self, rhs: T) -> Self {
        Self {
            left_leg: self.left_leg / rhs.clone(),
            right_leg: self.right_leg / rhs,
        }
    }
}

impl<T> Div for LegJoints<T>
where
    T: Div<Output = T>,
{
    type Output = Self;

    fn div(self, rhs: Self) -> Self {
        Self {
            left_leg: self.left_leg / rhs.left_leg,
            right_leg: self.right_leg / rhs.right_leg,
        }
    }
}
