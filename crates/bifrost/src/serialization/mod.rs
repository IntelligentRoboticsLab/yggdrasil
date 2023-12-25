//! Message protocol implementation based on serialization and deserialization.
mod codec;

pub use codec::{Decode, Encode, VarInt};

/// Derive macro to implement the [Decode] trait for structs and enums.
///
/// All the fields will be decoded individually in the same order they are defined in the struct or enum.
/// When decoding an enum, the variant id will be decoded first, which indicates to which variant
/// the data will be decoded. Then the fields of the variant will be decoded individually.
/// The size of the variant id is the smallest possible integer which can still store all the unique variant id's.
///
/// Because all the fields will be decoded individually, they all need to implement the [Decode] trait.
///
/// ## Examples
/// ```no_run
/// use std::io::Read;
/// use bifrost::{Result, serialization::Decode};
///
/// #[derive(Decode)]
/// struct Foo {
///     bar1: i32,
///     bar2: [u8; 4],
///     bar3: Vec<u8>,
/// }
/// ```
///
/// ## Examples
/// ```no_run
/// use std::io::Read;
/// use bifrost::{Result, serialization::Decode};
///
/// #[derive(Decode)]
/// enum Foo {
///     Bar1(i32),
///     Bar2,
///     Bar3{ x: f32, y: f32 },
/// }
/// ```
pub use bifrost_derive::Decode;

/// Derive macro to implement the [Encode] trait for structs and enums.
///
/// All the fields will be encoded individually in the same order they are defined in the struct or enum.
/// Before encoding an enum variant, a variant id will be encoded which indicates as to which
/// variant the data should be decoded.
/// The size of the variant id is the smallest possible integer which can still store all the unique variant id's.
///
/// Because all the fields will be encoded individually, they all need to implement the [Encode] trait.
///
/// ## Examples
/// ```no_run
/// use std::io::Write;
/// use bifrost::{Result, serialization::Encode};
///
/// #[derive(Encode)]
/// struct Foo {
///     bar1: i32,
///     bar2: [u8; 4],
///     bar3: Vec<u8>,
/// }
/// ```
///
/// ## Examples
/// ```no_run
/// use std::io::Write;
/// use bifrost::{Result, serialization::Encode};
///
/// #[derive(Encode)]
/// enum Foo {
///     Bar1(i32),
///     Bar2,
///     Bar3{ x: f32, y: f32 },
/// }
/// ```
pub use bifrost_derive::Encode;
