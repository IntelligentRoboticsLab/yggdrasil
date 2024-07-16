//! Traits and types for defining transformations between spaces.

use std::fmt;
use std::marker::PhantomData;

use nalgebra as na;

use super::space::*;

/// A transform between a `T1` in `S1` into a `T2` in `S2`.
pub trait Transform<T1, S1, T2, S2>
where
    S1: SpaceOver<T1>,
    S2: SpaceOver<T2>,
{
    fn transform(&self, x: &InSpace<T1, S1>) -> InSpace<T2, S2>;
}

/// An inverse transform between a `T1` in `S1` into a `T2` in `S2`.
pub trait InverseTransform<T1, S1, T2, S2>
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

impl<T, S1, S2> From<T> for BetweenSpaces<T, S1, S2>
where
    S1: Space,
    S2: Space,
{
    /// Wrap a `T` into a `BetweenSpaces<T, S1, S2>`.
    fn from(inner: T) -> Self {
        Self {
            inner,
            phantom: PhantomData,
        }
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
    S1: Space + fmt::Debug,
    S2: Space + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:?} between {} and {}",
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

impl<S1, S2> Transform<na::Point2<f32>, S1, na::Point2<f32>, S2>
    for BetweenSpaces<na::Isometry2<f32>, S1, S2>
where
    S1: SpaceOver<na::Point2<f32>>,
    S2: SpaceOver<na::Point2<f32>>,
{
    fn transform(&self, x: &InSpace<na::Point2<f32>, S1>) -> InSpace<na::Point2<f32>, S2> {
        self.inner.transform_point(&x.inner).into()
    }
}

impl<S1, S2> InverseTransform<na::Point2<f32>, S1, na::Point2<f32>, S2>
    for BetweenSpaces<na::Isometry2<f32>, S1, S2>
where
    S1: SpaceOver<na::Point2<f32>>,
    S2: SpaceOver<na::Point2<f32>>,
{
    fn inverse_transform(&self, x: &InSpace<na::Point2<f32>, S2>) -> InSpace<na::Point2<f32>, S1> {
        self.inner.inverse_transform_point(&x.inner).into()
    }
}

impl<S1, S2> Transform<na::Vector2<f32>, S1, na::Vector2<f32>, S2>
    for BetweenSpaces<na::Isometry2<f32>, S1, S2>
where
    S1: SpaceOver<na::Vector2<f32>>,
    S2: SpaceOver<na::Vector2<f32>>,
{
    fn transform(&self, x: &InSpace<na::Vector2<f32>, S1>) -> InSpace<na::Vector2<f32>, S2> {
        self.inner.transform_vector(&x.inner).into()
    }
}

impl<S1, S2> InverseTransform<na::Vector2<f32>, S1, na::Vector2<f32>, S2>
    for BetweenSpaces<na::Isometry2<f32>, S1, S2>
where
    S1: SpaceOver<na::Vector2<f32>>,
    S2: SpaceOver<na::Vector2<f32>>,
{
    fn inverse_transform(
        &self,
        x: &InSpace<na::Vector2<f32>, S2>,
    ) -> InSpace<na::Vector2<f32>, S1> {
        self.inner.inverse_transform_vector(&x.inner).into()
    }
}

impl<S1, S2> Transform<na::Point3<f32>, S1, na::Point3<f32>, S2>
    for BetweenSpaces<na::Isometry3<f32>, S1, S2>
where
    S1: SpaceOver<na::Point3<f32>>,
    S2: SpaceOver<na::Point3<f32>>,
{
    fn transform(&self, x: &InSpace<na::Point3<f32>, S1>) -> InSpace<na::Point3<f32>, S2> {
        self.inner.transform_point(&x.inner).into()
    }
}

impl<S1, S2> InverseTransform<na::Point3<f32>, S1, na::Point3<f32>, S2>
    for BetweenSpaces<na::Isometry3<f32>, S1, S2>
where
    S1: SpaceOver<na::Point3<f32>>,
    S2: SpaceOver<na::Point3<f32>>,
{
    fn inverse_transform(&self, x: &InSpace<na::Point3<f32>, S2>) -> InSpace<na::Point3<f32>, S1> {
        self.inner.inverse_transform_point(&x.inner).into()
    }
}

impl<S1, S2> Transform<na::Vector3<f32>, S1, na::Vector3<f32>, S2>
    for BetweenSpaces<na::Isometry3<f32>, S1, S2>
where
    S1: SpaceOver<na::Vector3<f32>>,
    S2: SpaceOver<na::Vector3<f32>>,
{
    fn transform(&self, x: &InSpace<na::Vector3<f32>, S1>) -> InSpace<na::Vector3<f32>, S2> {
        self.inner.transform_vector(&x.inner).into()
    }
}

impl<S1, S2> InverseTransform<na::Vector3<f32>, S1, na::Vector3<f32>, S2>
    for BetweenSpaces<na::Isometry3<f32>, S1, S2>
where
    S1: SpaceOver<na::Vector3<f32>>,
    S2: SpaceOver<na::Vector3<f32>>,
{
    fn inverse_transform(
        &self,
        x: &InSpace<na::Vector3<f32>, S2>,
    ) -> InSpace<na::Vector3<f32>, S1> {
        self.inner.inverse_transform_vector(&x.inner).into()
    }
}
