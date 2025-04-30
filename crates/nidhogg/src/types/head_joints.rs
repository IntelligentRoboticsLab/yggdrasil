use nidhogg_derive::{Builder, Filler};

use std::ops::{Add, Div, Mul, Sub};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Wrapper struct containing the head joints of the robot.
#[derive(Builder, Clone, Debug, Default, Filler, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct HeadJoints<T> {
    pub yaw: T,
    pub pitch: T,
}

impl<T> HeadJoints<T> {
    /// Transforms each element in the [`HeadJoints`] using the provided closure `f`,
    /// producing a new [`HeadJoints`] with the transformed values.
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::HeadJoints;
    /// use nidhogg::types::FillExt;
    ///
    /// let joints = HeadJoints::<u32>::default();
    ///
    /// let transformed = joints.map(|x| x + 1);
    ///
    /// assert_eq!(transformed, HeadJoints::fill(1));
    /// ```
    pub fn map<F, U>(self, mut f: F) -> HeadJoints<U>
    where
        F: FnMut(T) -> U,
    {
        HeadJoints {
            yaw: f(self.yaw),
            pitch: f(self.pitch),
        }
    }

    /// Zips two [`HeadJoints`] instances element-wise, creating a new [`HeadJoints`]
    /// containing tuples of corresponding elements from the two arrays.
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::HeadJoints;
    /// use nidhogg::types::FillExt;
    ///
    /// let zipped = HeadJoints::<u32>::default().zip(HeadJoints::<u32>::default());
    ///
    /// assert_eq!(zipped, HeadJoints::<(u32, u32)>::fill((0_u32, 0_u32)));
    /// ```
    pub fn zip<U>(self, other: HeadJoints<U>) -> HeadJoints<(T, U)> {
        HeadJoints {
            yaw: (self.yaw, other.yaw),
            pitch: (self.pitch, other.pitch),
        }
    }

    /// Return an iterator over references to the elements of the [`HeadJoints`].
    ///
    /// # Example
    ///
    /// ```
    /// use nidhogg::types::HeadJoints;
    ///
    /// let joints = HeadJoints::<f32>::default();
    /// for joint in joints.iter() {
    ///     assert_eq!(joint, 0.0);
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        [&self.yaw, &self.pitch].into_iter()
    }
}

impl<T> Add<T> for HeadJoints<T>
where
    T: Add<Output = T> + Clone,
{
    type Output = Self;

    fn add(self, rhs: T) -> Self {
        Self {
            yaw: self.yaw + rhs.clone(),
            pitch: self.pitch + rhs,
        }
    }
}

impl<T> Add<HeadJoints<T>> for HeadJoints<T>
where
    T: Add<T, Output = T>,
{
    type Output = Self;

    fn add(self, rhs: HeadJoints<T>) -> Self {
        Self {
            yaw: self.yaw + rhs.yaw,
            pitch: self.pitch + rhs.pitch,
        }
    }
}

impl<T> Sub<T> for HeadJoints<T>
where
    T: Sub<Output = T> + Clone,
{
    type Output = Self;

    fn sub(self, rhs: T) -> Self {
        Self {
            yaw: self.yaw - rhs.clone(),
            pitch: self.pitch - rhs,
        }
    }
}

impl<T> Sub<HeadJoints<T>> for HeadJoints<T>
where
    T: Sub<T, Output = T>,
{
    type Output = Self;

    fn sub(self, rhs: HeadJoints<T>) -> Self {
        Self {
            yaw: self.yaw - rhs.yaw,
            pitch: self.pitch - rhs.pitch,
        }
    }
}

impl<T> Mul<T> for HeadJoints<T>
where
    T: Mul<Output = T> + Clone,
{
    type Output = Self;

    fn mul(self, rhs: T) -> Self {
        Self {
            yaw: self.yaw * rhs.clone(),
            pitch: self.pitch * rhs,
        }
    }
}

impl<T> Mul<HeadJoints<T>> for HeadJoints<T>
where
    T: Mul<T, Output = T>,
{
    type Output = Self;

    fn mul(self, rhs: HeadJoints<T>) -> Self {
        Self {
            yaw: self.yaw * rhs.yaw,
            pitch: self.pitch * rhs.pitch,
        }
    }
}

impl<T> Div<T> for HeadJoints<T>
where
    T: Div<Output = T> + Clone,
{
    type Output = Self;

    fn div(self, rhs: T) -> Self {
        Self {
            yaw: self.yaw / rhs.clone(),
            pitch: self.pitch / rhs,
        }
    }
}

impl<T> Div<HeadJoints<T>> for HeadJoints<T>
where
    T: Div<T, Output = T>,
{
    type Output = Self;

    fn div(self, rhs: HeadJoints<T>) -> Self {
        Self {
            yaw: self.yaw / rhs.yaw,
            pitch: self.pitch / rhs.pitch,
        }
    }
}
