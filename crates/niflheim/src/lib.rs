//! # Niflheim: A Library for Spatial Transformations
//!
//! Niflheim is a library designed to facilitate transformations between different spaces. It
//! provides a set of traits and types to handle transformations of points and vectors between
//! various spaces, leveraging the type system to track which space such types are expressed in.
//!
//! ## [`Space`] and [`SpaceOver<T>`]
//!
//! At the core of Niflheim is the [`Space`] trait, which marks a type as representing a space.
//! Typically, such types are Zero-Sized Types and are never constructed.
//!
//! On its own, [`Space`] isn't very useful, since Niflheim enforces which spaces can contain which
//! types. To mark a space as containing a type, implement [`SpaceOver<T>`].
//!
//! ### Example
//!
//! ```rust
//! use nalgebra as na;
//! use niflheim::{Space, SpaceOver};
//!
//! struct LocalSpace;
//!
//! // Mark `LocalSpace` as representing a space.
//! impl Space for LocalSpace {}
//! // Mark `LocalSpace` as containing 3D points.
//! impl SpaceOver<na::Point3<f32>> for LocalSpace {}
//!
//! struct WorldSpace;
//!
//! // Mark `WorldSpace` as representing a space.
//! impl Space for WorldSpace {}
//! // Mark `WorldSpace` as containing 3D points.
//! impl SpaceOver<na::Point3<f32>> for WorldSpace {}
//! ```
//!
//! ## [`InSpace<T, S>`] and [`BetweenSpaces<T, S1, S2>`]
//!
//! Niflheim introduces two wrapper types to keep track of which spaces a value is expressed in,
//! [`InSpace<T, S>`] and [`BetweenSpaces<T, S1, S2>`]. These types can be constucted through
//! either their [`From<T>`] implementation.
//!
//! ### Example
//!
//! ```rust
//! # use nalgebra as na;
//! # use niflheim::{Space, SpaceOver};
//! # struct LocalSpace;
//! # impl Space for LocalSpace {}
//! # impl SpaceOver<na::Point3<f32>> for LocalSpace {}
//! # struct WorldSpace;
//! # impl Space for WorldSpace {}
//! # impl SpaceOver<na::Point3<f32>> for WorldSpace {}
//! use niflheim::types::*;
//!
//! // `Isometry3<S1, S2>` is an alias for `BetweenSpaces<na::Isometry3<f32>, S1, S2>`.
//! let tf: Isometry3<LocalSpace, WorldSpace> =
//!     na::Isometry3::new(na::vector![1., 2., 3.], na::vector![0., 0., 0.]).into();
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
//! To facilitate transformations using the wrapper types, the traits [`Transform`] and
//! [`InverseTransform`] are implemented for [`BetweenSpaces<T, S1, S2>`], where `T` are common
//! [`nalgebra`] types such as [`nalgebra::Isometry3<T>`].
//!
//! ### Example
//!
//! ```rust
//! # use nalgebra as na;
//! # use niflheim::{Space, SpaceOver};
//! # struct LocalSpace;
//! # impl Space for LocalSpace {}
//! # impl SpaceOver<na::Point3<f32>> for LocalSpace {}
//! # struct WorldSpace;
//! # impl Space for WorldSpace {}
//! # impl SpaceOver<na::Point3<f32>> for WorldSpace {}
//! # use niflheim::types::*;
//! # let tf: Isometry3<LocalSpace, WorldSpace> = na::Isometry3::new(
//! #     na::vector![1., 2., 3.],
//! #     na::vector![0., 0., 0.],
//! # ).into();
//! # let p1: Point3<LocalSpace> = na::point![1., 0., 0.].into();
//! use niflheim::Transform;
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

pub mod types;
