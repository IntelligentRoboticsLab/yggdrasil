//! Convenience type aliases for wrapped [`nalgebra`] types.

use nalgebra as na;

use super::space::InSpace;
use super::transform::BetweenSpaces;

pub type Point2<S> = InSpace<na::Point2<f32>, S>;
pub type Point3<S> = InSpace<na::Point3<f32>, S>;

pub type Vector2<S> = InSpace<na::Vector2<f32>, S>;
pub type Vector3<S> = InSpace<na::Vector3<f32>, S>;

pub type Isometry2<S1, S2> = BetweenSpaces<na::Isometry2<f32>, S1, S2>;
pub type Isometry3<S1, S2> = BetweenSpaces<na::Isometry3<f32>, S1, S2>;

#[macro_export]
macro_rules! point2 {
    () => {
        ::spatial::point2!(as _)
    };
    (as $space:ty) => {
        ::spatial::types::Point2::<$space>::new(::nalgebra::Point2::origin())
    };
    ($x:expr, $y:expr) => {
        ::spatial::point2!(as _, $x, $y)
    };
    (as $space:ty, $x:expr, $y:expr) => {
        ::spatial::types::Point2::<$space>::new(::nalgebra::Point2::new($x, $y))
    };
}

#[macro_export]
macro_rules! point3 {
    () => {
        ::spatial::point3!(as _)
    };
    (as $space:ty) => {
        ::spatial::types::Point3::<$space>::new(::nalgebra::Point3::origin())
    };
    ($x:expr, $y:expr, $z:expr) => {
        ::spatial::point3!(as _, $x, $y, $z)
    };
    (as $space:ty, $x:expr, $y:expr, $z:expr) => {
        ::spatial::types::Point3::<$space>::new(::nalgebra::Point3::new($x, $y, $z))
    };
}
