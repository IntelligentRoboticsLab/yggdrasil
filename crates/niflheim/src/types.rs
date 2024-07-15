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
