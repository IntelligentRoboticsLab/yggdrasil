//! Traits and types for defining spaces and tagging data with spaces.

use std::fmt;
use std::marker::PhantomData;
use std::ops::{Add, AddAssign, Deref, DerefMut, Div, DivAssign, Mul, MulAssign, Sub, SubAssign};

/// Marker trait for spaces which can be mapped between.
pub trait Space {}

/// Marker trait for spaces which contain `T`s within them.
pub trait SpaceOver<T>: Space {}

/// Wrapper type for tagging a `T` as existing in `S`.
pub struct InSpace<T, S: SpaceOver<T>> {
    pub inner: T,
    phantom: PhantomData<S>,
}

impl<T, S: SpaceOver<T>> InSpace<T, S> {
    pub const fn new(inner: T) -> Self {
        Self {
            inner,
            phantom: PhantomData,
        }
    }
}

impl<T, S: SpaceOver<T>> From<T> for InSpace<T, S> {
    /// Wrap a `T` into a `InSpace<T, S>`.
    fn from(inner: T) -> Self {
        Self::new(inner)
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

impl<T, S: SpaceOver<T>> Deref for InSpace<T, S> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T, S: SpaceOver<T>> DerefMut for InSpace<T, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: Eq, S: SpaceOver<T>> Eq for InSpace<T, S> {}

impl<T: PartialEq, S: SpaceOver<T>> PartialEq for InSpace<T, S> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl<T1, T2, S> Add<InSpace<T2, S>> for InSpace<T1, S>
where
    T1: Add<T2>,
    S: SpaceOver<T1> + SpaceOver<T2> + SpaceOver<T1::Output>,
{
    type Output = InSpace<T1::Output, S>;

    fn add(self, rhs: InSpace<T2, S>) -> Self::Output {
        (self.inner + rhs.inner).into()
    }
}

impl<'a, T1, T2, S> Add<&'a InSpace<T2, S>> for InSpace<T1, S>
where
    T1: Add<&'a T2>,
    S: SpaceOver<T1> + SpaceOver<T2> + SpaceOver<T1::Output>,
{
    type Output = InSpace<T1::Output, S>;

    fn add(self, rhs: &'a InSpace<T2, S>) -> Self::Output {
        (self.inner + &rhs.inner).into()
    }
}

impl<'a, T1, T2, S> Add<InSpace<T2, S>> for &'a InSpace<T1, S>
where
    &'a T1: Add<T2>,
    S: SpaceOver<T1> + SpaceOver<T2> + SpaceOver<<&'a T1 as Add<T2>>::Output>,
{
    type Output = InSpace<<&'a T1 as Add<T2>>::Output, S>;

    fn add(self, rhs: InSpace<T2, S>) -> Self::Output {
        (&self.inner + rhs.inner).into()
    }
}

impl<'a, T1, T2, S> Add<&'a InSpace<T2, S>> for &'a InSpace<T1, S>
where
    &'a T1: Add<&'a T2>,
    S: SpaceOver<T1> + SpaceOver<T2> + SpaceOver<<&'a T1 as Add<&'a T2>>::Output>,
{
    type Output = InSpace<<&'a T1 as Add<&'a T2>>::Output, S>;

    fn add(self, rhs: &'a InSpace<T2, S>) -> Self::Output {
        (&self.inner + &rhs.inner).into()
    }
}

impl<T1, T2, S> AddAssign<InSpace<T2, S>> for InSpace<T1, S>
where
    T1: AddAssign<T2>,
    S: SpaceOver<T1> + SpaceOver<T2>,
{
    fn add_assign(&mut self, rhs: InSpace<T2, S>) {
        self.inner += rhs.inner;
    }
}

impl<'a, T1, T2, S> AddAssign<&'a InSpace<T2, S>> for InSpace<T1, S>
where
    T1: AddAssign<&'a T2>,
    S: SpaceOver<T1> + SpaceOver<T2>,
{
    fn add_assign(&mut self, rhs: &'a InSpace<T2, S>) {
        self.inner += &rhs.inner;
    }
}

impl<T1, T2, S> Sub<InSpace<T2, S>> for InSpace<T1, S>
where
    T1: Sub<T2>,
    S: SpaceOver<T1> + SpaceOver<T2> + SpaceOver<T1::Output>,
{
    type Output = InSpace<T1::Output, S>;

    fn sub(self, rhs: InSpace<T2, S>) -> Self::Output {
        (self.inner - rhs.inner).into()
    }
}

impl<'a, T1, T2, S> Sub<&'a InSpace<T2, S>> for InSpace<T1, S>
where
    T1: Sub<&'a T2>,
    S: SpaceOver<T1> + SpaceOver<T2> + SpaceOver<T1::Output>,
{
    type Output = InSpace<T1::Output, S>;

    fn sub(self, rhs: &'a InSpace<T2, S>) -> Self::Output {
        (self.inner - &rhs.inner).into()
    }
}

impl<'a, T1, T2, S> Sub<InSpace<T2, S>> for &'a InSpace<T1, S>
where
    &'a T1: Sub<T2>,
    S: SpaceOver<T1> + SpaceOver<T2> + SpaceOver<<&'a T1 as Sub<T2>>::Output>,
{
    type Output = InSpace<<&'a T1 as Sub<T2>>::Output, S>;

    fn sub(self, rhs: InSpace<T2, S>) -> Self::Output {
        (&self.inner - rhs.inner).into()
    }
}

impl<'a, T1, T2, S> Sub<&'a InSpace<T2, S>> for &'a InSpace<T1, S>
where
    &'a T1: Sub<&'a T2>,
    S: SpaceOver<T1> + SpaceOver<T2> + SpaceOver<<&'a T1 as Sub<&'a T2>>::Output>,
{
    type Output = InSpace<<&'a T1 as Sub<&'a T2>>::Output, S>;

    fn sub(self, rhs: &'a InSpace<T2, S>) -> Self::Output {
        (&self.inner - &rhs.inner).into()
    }
}

impl<T1, T2, S> SubAssign<InSpace<T2, S>> for InSpace<T1, S>
where
    T1: SubAssign<T2>,
    S: SpaceOver<T1> + SpaceOver<T2>,
{
    fn sub_assign(&mut self, rhs: InSpace<T2, S>) {
        self.inner -= rhs.inner;
    }
}

impl<'a, T1, T2, S> SubAssign<&'a InSpace<T2, S>> for InSpace<T1, S>
where
    T1: SubAssign<&'a T2>,
    S: SpaceOver<T1> + SpaceOver<T2>,
{
    fn sub_assign(&mut self, rhs: &'a InSpace<T2, S>) {
        self.inner -= &rhs.inner;
    }
}

impl<T1, T2, S> Mul<InSpace<T2, S>> for InSpace<T1, S>
where
    T1: Mul<T2>,
    S: SpaceOver<T1> + SpaceOver<T2> + SpaceOver<T1::Output>,
{
    type Output = InSpace<T1::Output, S>;

    fn mul(self, rhs: InSpace<T2, S>) -> Self::Output {
        (self.inner * rhs.inner).into()
    }
}

impl<'a, T1, T2, S> Mul<&'a InSpace<T2, S>> for InSpace<T1, S>
where
    T1: Mul<&'a T2>,
    S: SpaceOver<T1> + SpaceOver<T2> + SpaceOver<T1::Output>,
{
    type Output = InSpace<T1::Output, S>;

    fn mul(self, rhs: &'a InSpace<T2, S>) -> Self::Output {
        (self.inner * &rhs.inner).into()
    }
}

impl<'a, T1, T2, S> Mul<InSpace<T2, S>> for &'a InSpace<T1, S>
where
    &'a T1: Mul<T2>,
    S: SpaceOver<T1> + SpaceOver<T2> + SpaceOver<<&'a T1 as Mul<T2>>::Output>,
{
    type Output = InSpace<<&'a T1 as Mul<T2>>::Output, S>;

    fn mul(self, rhs: InSpace<T2, S>) -> Self::Output {
        (&self.inner * rhs.inner).into()
    }
}

impl<'a, T1, T2, S> Mul<&'a InSpace<T2, S>> for &'a InSpace<T1, S>
where
    &'a T1: Mul<&'a T2>,
    S: SpaceOver<T1> + SpaceOver<T2> + SpaceOver<<&'a T1 as Mul<&'a T2>>::Output>,
{
    type Output = InSpace<<&'a T1 as Mul<&'a T2>>::Output, S>;

    fn mul(self, rhs: &'a InSpace<T2, S>) -> Self::Output {
        (&self.inner * &rhs.inner).into()
    }
}

impl<T, S> Mul<f32> for InSpace<T, S>
where
    T: Mul<f32, Output = T>,
    S: SpaceOver<T>,
{
    type Output = InSpace<T, S>;

    fn mul(self, rhs: f32) -> Self::Output {
        (self.inner * rhs).into()
    }
}

// TODO: This code summons eldritch horrors into the compiler starting from 1.82. These demons are
// mostly harmless, however, they dislike you multiplying floats inside an `impl Iterator<Item=f32>`.
impl<T, S> Mul<InSpace<T, S>> for f32
where
    f32: Mul<T, Output = T>,
    S: SpaceOver<T>,
{
    type Output = InSpace<T, S>;

    fn mul(self, rhs: InSpace<T, S>) -> Self::Output {
        (self * rhs.inner).into()
    }
}

impl<T1, T2, S> MulAssign<InSpace<T2, S>> for InSpace<T1, S>
where
    T1: MulAssign<T2>,
    S: SpaceOver<T1> + SpaceOver<T2>,
{
    fn mul_assign(&mut self, rhs: InSpace<T2, S>) {
        self.inner *= rhs.inner;
    }
}

impl<'a, T1, T2, S> MulAssign<&'a InSpace<T2, S>> for InSpace<T1, S>
where
    T1: MulAssign<&'a T2>,
    S: SpaceOver<T1> + SpaceOver<T2>,
{
    fn mul_assign(&mut self, rhs: &'a InSpace<T2, S>) {
        self.inner *= &rhs.inner;
    }
}

impl<T1, T2, S> Div<InSpace<T2, S>> for InSpace<T1, S>
where
    T1: Div<T2>,
    S: SpaceOver<T1> + SpaceOver<T2> + SpaceOver<T1::Output>,
{
    type Output = InSpace<T1::Output, S>;

    fn div(self, rhs: InSpace<T2, S>) -> Self::Output {
        (self.inner / rhs.inner).into()
    }
}

impl<'a, T1, T2, S> Div<&'a InSpace<T2, S>> for InSpace<T1, S>
where
    T1: Div<&'a T2>,
    S: SpaceOver<T1> + SpaceOver<T2> + SpaceOver<T1::Output>,
{
    type Output = InSpace<T1::Output, S>;

    fn div(self, rhs: &'a InSpace<T2, S>) -> Self::Output {
        (self.inner / &rhs.inner).into()
    }
}

impl<'a, T1, T2, S> Div<InSpace<T2, S>> for &'a InSpace<T1, S>
where
    &'a T1: Div<T2>,
    S: SpaceOver<T1> + SpaceOver<T2> + SpaceOver<<&'a T1 as Div<T2>>::Output>,
{
    type Output = InSpace<<&'a T1 as Div<T2>>::Output, S>;

    fn div(self, rhs: InSpace<T2, S>) -> Self::Output {
        (&self.inner / rhs.inner).into()
    }
}

impl<'a, T1, T2, S> Div<&'a InSpace<T2, S>> for &'a InSpace<T1, S>
where
    &'a T1: Div<&'a T2>,
    S: SpaceOver<T1> + SpaceOver<T2> + SpaceOver<<&'a T1 as Div<&'a T2>>::Output>,
{
    type Output = InSpace<<&'a T1 as Div<&'a T2>>::Output, S>;

    fn div(self, rhs: &'a InSpace<T2, S>) -> Self::Output {
        (&self.inner / &rhs.inner).into()
    }
}

impl<T, S> Div<f32> for InSpace<T, S>
where
    T: Div<f32, Output = T>,
    S: SpaceOver<T>,
{
    type Output = InSpace<T, S>;

    fn div(self, rhs: f32) -> Self::Output {
        (self.inner / rhs).into()
    }
}

impl<T1, T2, S> DivAssign<InSpace<T2, S>> for InSpace<T1, S>
where
    T1: DivAssign<T2>,
    S: SpaceOver<T1> + SpaceOver<T2>,
{
    fn div_assign(&mut self, rhs: InSpace<T2, S>) {
        self.inner /= rhs.inner;
    }
}

impl<'a, T1, T2, S> DivAssign<&'a InSpace<T2, S>> for InSpace<T1, S>
where
    T1: DivAssign<&'a T2>,
    S: SpaceOver<T1> + SpaceOver<T2>,
{
    fn div_assign(&mut self, rhs: &'a InSpace<T2, S>) {
        self.inner /= &rhs.inner;
    }
}
