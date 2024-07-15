//! Traits and types for defining spaces and tagging data with spaces.

use std::fmt;
use std::marker::PhantomData;

/// Marker trait for spaces which can be mapped between.
pub trait Space {}

/// Marker trait for spaces which contain `T`s within them.
pub trait SpaceOver<T>: Space {}

/// Wrapper type for tagging a `T` as existing in `S`.
pub struct InSpace<T, S: SpaceOver<T>> {
    pub inner: T,
    phantom: PhantomData<S>,
}

impl<T, S: SpaceOver<T>> From<T> for InSpace<T, S> {
    /// Wrap a `T` into a `InSpace<T, S>`.
    fn from(inner: T) -> Self {
        Self {
            inner,
            phantom: PhantomData,
        }
    }
}

impl<T: Clone, S: SpaceOver<T>> Clone for InSpace<T, S> {
    fn clone(&self) -> Self {
        self.inner.clone().into()
    }
}

impl<T: Copy, S: SpaceOver<T>> Copy for InSpace<T, S> {}

impl<T, S> fmt::Debug for InSpace<T, S>
where
    T: fmt::Debug,
    S: SpaceOver<T>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} in {}", self.inner, std::any::type_name::<S>())
    }
}

impl<T: Default, S: SpaceOver<T>> Default for InSpace<T, S> {
    fn default() -> Self {
        T::default().into()
    }
}

impl<T: Eq, S: SpaceOver<T>> Eq for InSpace<T, S> {}

impl<T: PartialEq, S: SpaceOver<T>> PartialEq for InSpace<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
