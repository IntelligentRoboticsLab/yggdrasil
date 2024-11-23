//! Traits and types for defining transformations between spaces.

pub use spatial_derive::Transform;

use std::fmt;
use std::marker::PhantomData;
use std::ops::Mul;

use nalgebra as na;

use super::space::{InSpace, Space, SpaceOver};

/// A transform between a `T1` in `S1` into a `T2` in `S2`.
pub trait Transform<T1, T2, S1, S2>
where
    S1: SpaceOver<T1>,
    S2: SpaceOver<T2>,
{
    fn transform(&self, x: &InSpace<T1, S1>) -> InSpace<T2, S2>;
}

/// An inverse transform between a `T1` in `S1` into a `T2` in `S2`.
pub trait InverseTransform<T1, T2, S1, S2>
where
    S1: SpaceOver<T1>,
    S2: SpaceOver<T2>,
{
    fn inverse_transform(&self, x: &InSpace<T2, S2>) -> InSpace<T1, S1>;
}

/// Wrapper type for `T`s which can be used to transform between `S1` and `S2`.
pub struct BetweenSpaces<T, S1, S2>
where
    S1: Space,
    S2: Space,
{
    pub inner: T,
    phantom: PhantomData<(S1, S2)>,
}

impl<T, S1: Space, S2: Space> BetweenSpaces<T, S1, S2> {
    /// Wrap a `T` into a `BetweenSpaces<T, S1, S2>`.
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            phantom: PhantomData,
        }
    }

    pub fn chain<T2, S3>(self, other: BetweenSpaces<T2, S2, S3>) -> BetweenSpaces<T2::Output, S1, S3>
    where
        T2: Mul<T>,
        S3: Space,
    {
        BetweenSpaces::new(other.inner * self.inner)
    }

    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> BetweenSpaces<U, S1, S2> {
        BetweenSpaces::new(f(self.inner))
    }

    pub fn as_ref(&self) -> BetweenSpaces<&T, S1, S2> {
        BetweenSpaces::new(&self.inner)
    }

    pub fn as_mut(&mut self) -> BetweenSpaces<&mut T, S1, S2> {
        BetweenSpaces::new(&mut self.inner)
    }
}

impl<T, S1: Space, S2: Space> From<T> for BetweenSpaces<T, S1, S2> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: Clone, S1, S2> Clone for BetweenSpaces<T, S1, S2>
where
    S1: Space,
    S2: Space,
{
    fn clone(&self) -> Self {
        self.inner.clone().into()
    }
}

impl<T: Copy, S1, S2> Copy for BetweenSpaces<T, S1, S2>
where
    S1: Space,
    S2: Space,
{
}

impl<T, S1, S2> fmt::Debug for BetweenSpaces<T, S1, S2>
where
    T: fmt::Debug,
    S1: Space,
    S2: Space,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} ({} -> {})",
            self.inner,
            std::any::type_name::<S1>(),
            std::any::type_name::<S2>(),
        )
    }
}

impl<T: Default, S1, S2> Default for BetweenSpaces<T, S1, S2>
where
    S1: Space,
    S2: Space,
{
    fn default() -> Self {
        T::default().into()
    }
}

impl<T: Eq, S1, S2> Eq for BetweenSpaces<T, S1, S2>
where
    S1: Space,
    S2: Space,
{
}

impl<T: PartialEq, S1, S2> PartialEq for BetweenSpaces<T, S1, S2>
where
    S1: Space,
    S2: Space,
{
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

macro_rules! impl_transform {
    ($transform:ty, $inner:ty, $forward:ident, $inverse:ident) => {
        impl<S1, S2> Transform<$inner, $inner, S1, S2> for BetweenSpaces<$transform, S1, S2>
        where
            S1: SpaceOver<$inner>,
            S2: SpaceOver<$inner>,
        {
            fn transform(&self, x: &InSpace<$inner, S1>) -> InSpace<$inner, S2> {
                InSpace::new(self.inner.$forward(&x.inner))
            }
        }

        impl<S1, S2> InverseTransform<$inner, $inner, S1, S2> for BetweenSpaces<$transform, S1, S2>
        where
            S1: SpaceOver<$inner>,
            S2: SpaceOver<$inner>,
        {
            fn inverse_transform(&self, x: &InSpace<$inner, S2>) -> InSpace<$inner, S1> {
                InSpace::new(self.inner.$inverse(&x.inner))
            }
        }
    };
}

impl_transform!(
    na::Isometry2<f32>,
    na::Point2<f32>,
    transform_point,
    inverse_transform_point
);
impl_transform!(
    na::Isometry2<f32>,
    na::Vector2<f32>,
    transform_vector,
    inverse_transform_vector
);
impl_transform!(na::Isometry2<f32>, na::Isometry2<f32>, mul, inv_mul);

impl_transform!(
    na::Isometry3<f32>,
    na::Point3<f32>,
    transform_point,
    inverse_transform_point
);
impl_transform!(
    na::Isometry3<f32>,
    na::Vector3<f32>,
    transform_vector,
    inverse_transform_vector
);
impl_transform!(na::Isometry3<f32>, na::Isometry3<f32>, mul, inv_mul);
