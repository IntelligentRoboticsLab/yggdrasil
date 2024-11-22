//! # spatial 🌐
//!
//! spatial is a library designed for transformations between coordinate frames using Rust's
//! powerful type system.
//!
//! It provides a set of wrapper types for common [`nalgebra`] types such as
//! [`nalgebra::Vector2`] and [`nalgebra::Point3`] that take additional type arguments to define the coordinate frame it is in.
//! For transformation types such as [`nalgebra::Isometry3`] the wrapper contains two additional type arguments to denote
//! a transformation between spaces.
//!
//! ## [`Space`] and [`SpaceOver<T>`]
//!
//! The main trait in spatial is [`Space`]. This trait is implemented for marker types that represent
//! a coordinate frame. On its own, this trait is not very useful, but when combined with [`SpaceOver<T>`]
//! it can be used to define which types are valid in a given space.
//!
//! ### Example
//!
//! ```rust
//! #use nalgebra as na;
//! #use spatial::{Space, SpaceOver};
//!
//! // Define marker types for different spaces.
//! struct LocalSpace;
//! struct WorldSpace;
//!
//! // Mark both `LocalSpace` and `WorldSpace` as representing a space.
//! impl Space for LocalSpace {}
//! impl Space for WorldSpace {}
//!
//! // Using `SpaceOver<T>` we mark both spaces as containing 3D points.
//! impl SpaceOver<na::Point3<f32>> for LocalSpace {}
//! impl SpaceOver<na::Point3<f32>> for WorldSpace {}
//! ```
//!
//! ## [`InSpace<T, S>`] and [`BetweenSpaces<T, S1, S2>`]
//!
//! spatial introduces the [`InSpace<T, S>`] to denote which space a value is expressed in.
//! This type wraps a type `T` and denotes that it is in the space `S`.
//!
//! To transform between spaces, spatial provides the [`BetweenSpaces<T, S1, S2>`] type.
//! This type wraps a type `T` and denotes that it is a transformation between spaces `S1` and `S2`.
//!
//! ### Example
//!
//! ```rust
//! # use nalgebra as na;
//! # use nalgebra::vector;
//! # use spatial::{Space, SpaceOver};
//! # struct LocalSpace;
//! # impl Space for LocalSpace {}
//! # impl SpaceOver<na::Point3<f32>> for LocalSpace {}
//! # struct WorldSpace;
//! # impl Space for WorldSpace {}
//! # impl SpaceOver<na::Point3<f32>> for WorldSpace {}
//! use spatial::types::*;
//!
//! // `Isometry3<S1, S2>` is an alias for `BetweenSpaces<na::Isometry3<f32>, S1, S2>`.
//! let tf: Isometry3<LocalSpace, WorldSpace> =
//!     na::Isometry3::new(vector![1., 2., 3.], vector![0., 0., 0.]).into();
//!
//! // `Point3<S>` is an alias for `InSpace<na::Point3<f32>, S>`.
//! let p1: Point3<LocalSpace> = na::point![1., 0., 0.].into();
//!
//! // ERROR: `p1` is in local space!
//! // let p2: Point3<WorldSpace> = p1;
//! let p2: Point3<LocalSpace> = p1;
//!
//! assert_eq!(p1, p2);
//! ```
//!
//! ## [`Transform`] and [`InverseTransform`]
//!
//! To support transformations using the wrapper types, the traits [`Transform`] and
//! [`InverseTransform`] are implemented for [`BetweenSpaces<T, S1, S2>`], where `T` are common
//! [`nalgebra`] types such as [`nalgebra::Isometry3<T>`].
//!
//! //! ### Example
//!
//! ```rust
//! # use nalgebra as na;
//! # use spatial::{Space, SpaceOver};
//! # struct LocalSpace;
//! # impl Space for LocalSpace {}
//! # impl SpaceOver<na::Point3<f32>> for LocalSpace {}
//! # struct WorldSpace;
//! # impl Space for WorldSpace {}
//! # impl SpaceOver<na::Point3<f32>> for WorldSpace {}
//! # use spatial::types::*;
//! # let tf: Isometry3<LocalSpace, WorldSpace> = na::Isometry3::new(
//! #     na::vector![1., 2., 3.],
//! #     na::vector![0., 0., 0.],
//! # ).into();
//! # let p1: Point3<LocalSpace> = na::point![1., 0., 0.].into();
//! use spatial::Transform;
//!
//! // Using `tf` we can transform between these two spaces.
//! let p2: Point3<WorldSpace> = tf.transform(&p1);
//!
//! assert_eq!(p2.inner, na::point![2., 2., 3.]);
//! ```

pub mod space;
pub use space::*;

pub mod transform;
pub use transform::*;

#[macro_use]
pub mod types;
